# Analyzing the kernel

## ByteOS

### init memory - 

in file : iso / boot / grub / grub.cfg

set timeout=0
set default=0

menuentry "Byte OS" {
    multiboot2 /boot/byteos.elf
    boot
}

in file : kernel / mm / vmm.c

extern struct page_table p4_table; // Initial kernel p4 table
struct mmu_info kernel_mmu;

__attribute__((aligned(PAGE_SIZE))) const uint8_t zero_page[PAGE_SIZE] = { 0 };

void vmm_init(void)
{
	kernel_mmu.p4 = phys_to_kern((physaddr_t)&p4_table);
	kernel_mmu.areas = NULL;
	klog("vmm", "Kernel P4 at %p\n", kernel_mmu.p4);
//	vmm_dump_tables();
}

funcs:

static inline kernaddr_t phys_to_kern(physaddr_t p)
{
	if (p == (physaddr_t)NULL)
		return NULL;
	return (kernaddr_t)(p + KERNEL_TEXT_BASE);
}

# psudo code

```C
init kernel
init page_table

void init_memory() 
{
    kernel._page_table = page_table.get_physical_address() // TODO: how does it get the physicall address

}
```


### process memory creation -

in file : kernel / proc / sched.c

void sched_run_bsp(void)
{
	// Create the init task
	runq_init(&dummy);

	struct task *t = task_fork(&dummy, init_kernel, FORK_KTHREAD, NULL);
	task_wakeup(t);

	wait_for_schedulers();

	lapic_timer_enable();
	// TODO: Possible race condition here, if we get rescheduled before sched_yield happens
	sched_yield();
}

funcs:

// Initialises the run queue for the current CPU
void runq_init(struct task *initial_parent)
{
	struct runq *rq = kmalloc(sizeof(struct runq), KM_NONE);
	memset(rq, 0, sizeof *rq);
	percpu_set(run_queue, rq);

	// Create the idle task
	struct task *idle = task_fork(initial_parent, idle_task, TASK_KTHREAD, NULL);

	// Pin to this CPU
	cpuset_clear(&idle->affinity);
	cpuset_set_id(&idle->affinity, percpu_get(id), 1);
	cpuset_pin(&idle->affinity);
	
	// Set flags
	idle->state = TASK_S_RUNNABLE;
	idle->tid = TID_IDLE; // Idle task always has ID 0

	rq->idle = idle;
}

struct task *task_fork(struct task *parent, virtaddr_t entry, uint64_t clone_flags, const struct callee_regs *regs)
{
	if (parent == NULL)
		parent = current;

	kassert_dbg(!((clone_flags & FORK_KTHREAD) && (clone_flags & FORK_UTHREAD)));

	struct task *t = kmalloc(sizeof(struct task), KM_NONE);
	memset(t, 0, sizeof *t);
	t->flags = parent->flags;
	t->tid = atomic_inc_read32(&next_tid);
	t->tgid = t->tid; // TODO

	klog_verbose("task", "Forked PID %d to create PID %d\n", parent->tid, t->tid);

	// Allocate a kernel stack
	uintptr_t kstack = TASK_KSTACK_SIZE + (uintptr_t)page_to_virt(pmm_alloc_order(TASK_KSTACK_ORDER, GFP_NONE));
	uint64_t *stack = (uint64_t *)kstack;
	// TODO: Remove this variable. We can work out the stack top by masking rsp
	// given that the kernel stack size is fixed at compile time, and allocs are aligned.
	t->rsp_original = (virtaddr_t)kstack;

	// Copy MMU information and set up the kernel stack
	if (clone_flags & FORK_KTHREAD) {
		if (regs == NULL)
			regs = &default_regs;
		t->mmu = &kernel_mmu;	
		t->flags |= TASK_KTHREAD;
		*--stack = (uint64_t)entry;
		*--stack = regs->rbx; // Argument passed to the thread
		*--stack = (uint64_t)ret_from_kfork; // Where switch_to will return
	} else {
		kassert_dbg(regs != NULL);
		t->flags &= ~(TASK_KTHREAD);

		if (clone_flags & FORK_UTHREAD) {
			mmu_inc_users(parent->mmu);
			t->mmu = parent->mmu;
		} else {
			t->mmu = mmu_alloc();
			mmu_clone_cow(t->mmu, parent->mmu);
		}

		// Set up simulated iret frame
		*--stack = 0x20 | 0x3; // ss
		*--stack = regs->rsp; // rsp
		*--stack = read_rflags() | 0x200; // rflags with interrupts enabled
		*--stack = 0x28 | 0x3; // cs
		*--stack = (uint64_t)entry; // rip
		*--stack = (uint64_t)ret_from_ufork; // Where switch_to will return
	}

	*--stack = regs->rbx;
	*--stack = regs->rbp;
	*--stack = regs->r12;
	*--stack = regs->r13;
	*--stack = regs->r14;
	*--stack = regs->r15;
	t->rsp_top = (virtaddr_t)stack;

	cpuset_copy(&t->affinity, &parent->affinity);

	// Add the task to the scheduler
	t->state = TASK_S_NOT_STARTED;
	return t;
}

void task_wakeup(struct task *t)
{
	if (t->state != TASK_S_RUNNABLE) {
		t->state = TASK_S_RUNNABLE;
		sched_add(t);
	}

# psudo code
```C
void create_proccess() {

    Task task 
    task.init_cpu_run_queue()
    Task forked_task = fork_task() // create the proccess + memory allocate and resources
    forked_task.start() // start the proccess - adds it to the run queue
    wait_for_scheduler() // wait for permission
    lapic_timer_enable() // Enables the lapic timer with interrupts every 10ms
    if(forked_task._is_resceduled == false)
    {
        schedule_and_run()
    }
}
```

### page faults - 

in file : kernel / cpu / exeptions.c

static void page_fault_handler(struct isr_ctx *regs)
{
	uintptr_t faulting_address;
	asm volatile("mov %%cr2, %0" : "=r" (faulting_address));

	// The fault was likely due to an access in kernel space, so give up
	if (faulting_address & (1ULL << 63))
		goto kernel_panic;

	// If interrupts were enabled, we are safe to enable them again
	if (regs->rflags & 0x200)
		sti();

	if (current != NULL) {
		virtaddr_t aligned_addr = (virtaddr_t)(faulting_address & ~(PAGE_SIZE - 1));

		write_spin_lock(&current->mmu->pgtab_lock);

		pte_t *pte = vmm_get_pte(current->mmu, aligned_addr);
		bool done = regs->info & PAGE_FAULT_RW && cow_handle_write(pte, aligned_addr);

		write_spin_unlock(&current->mmu->pgtab_lock);

		if (done)
			return;
	}

	// TODO: Kill process

kernel_panic:
	// Otherwise, the write was not allowed, so we panic
	panic(
		"Page fault:\n"
		"\tfaulting address: %p\n"
		"\trip: %p, rsp: %p\n"
		"\terr_code: %lx",
		(void *)faulting_address,
		(void *)regs->rip, (void *)regs->rsp,
		(regs->info & 0xFFFFFFFF)
	);
}

# psudo code : 

```	c
/*
Error - data type which contains:
1.  error code and interrupt number for exceptions
2. syscall number for syscalls
3. interrupt number otherwise
4. interrupt stack frame 
etc.. all unsigned long long int
*/
void handlePageFault(Error error) 
{
	ptr* faulting_address;
	// The fault was likely due to an access in kernel space, so give up
	if(faulting_address.used_kernel_space() == true)
	{
		kernel_panic() // 
	}
	// If interrupts were enabled, we are safe to enable them again
	else if(error.interrupt_was_unabled() == true)
	{
		enable()
	}
	else if(current_task.is_active() == true)
	{
		VirtualAddress vadd = error.get_virtual_addr()

		write_spin_lock(current_process->mmu->page_table); // locks the process until finish use the resource

		Adderss addr = vmm_get_virtual_memory_addr()
		PageTable pt = vmm_get_page_table(current->mmu, addr);
		bool done = cow_handle_write(pt, addr); // Returns true if the write is allowed and should be retried

		write_spin_unlock(current_process->mmu->page_table);

		if (done)
			return;
	}
	else 
	{
		kill_process()
	}


}
```
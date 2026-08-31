//! Regression: block allocation must never materialise BLOCK_SIZE
//! payloads on the pushing thread's stack. 4 KiB payload x 128 slots is a
//! ~512 KiB frame under the old `Box::new(BlockNode { .. })`, which
//! overflows the 128 KiB thread stack below (held tight enough that debug-mode per-item copies still fit); the `alloc_zeroed` version
//! never touches the stack. (In production the victim was a may coroutine,
//! killed WITHOUT unwinding, corrupting the queue mid-push - the
//! PriceWhisperer BRRTRouter dispatcher wedge of 2026-08-31.)

#[derive(Clone)]
struct Big([u8; 4096]);

fn hammer<Q>(push: impl Fn(&Q, Big), pop: impl Fn(&Q) -> Option<Big>, q: Q)
where
    Q: Send + 'static,
{
    // Cross several 128-slot block boundaries.
    for round in 0..5 {
        for _ in 0..200 {
            push(&q, Big([round as u8; 4096]));
        }
        for _ in 0..200 {
            assert!(pop(&q).is_some(), "queue lost items");
        }
    }
}

fn on_small_stack(name: &str, f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .name(name.to_string())
        .stack_size(128 * 1024)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn mpsc_blocks_allocate_off_stack() {
    on_small_stack("mpsc-small-stack", || {
        let q = may_queue::mpsc::Queue::new();
        hammer(|q, v| q.push(v), |q| q.pop(), q);
    });
}

#[test]
fn spsc_blocks_allocate_off_stack() {
    on_small_stack("spsc-small-stack", || {
        let q = may_queue::spsc::Queue::new();
        hammer(|q, v| q.push(v), |q| q.pop(), q);
    });
}


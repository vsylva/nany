#![allow(unsafe_op_in_unsafe_fn)]
//! The registry in front of `naly`: hook, trampoline lookup, unhook, unhook_all.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

type Binop = unsafe extern "system" fn(i32, i32) -> i32;
type Unop = unsafe extern "system" fn(i32) -> i32;

static LOCK: Mutex<()> = Mutex::new(());
static HITS: AtomicUsize = AtomicUsize::new(0);
static ORIG: AtomicUsize = AtomicUsize::new(0);

fn lock() -> std::sync::MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[inline(never)]
#[unsafe(no_mangle)]
unsafe extern "system" fn registry_target(a: i32, b: i32) -> i32 {
    a.wrapping_sub(b)
}

#[inline(never)]
#[unsafe(no_mangle)]
unsafe extern "system" fn registry_second(a: i32) -> i32 {
    a.wrapping_add(7)
}

unsafe extern "system" fn registry_detour(a: i32, b: i32) -> i32 {
    HITS.fetch_add(1, SeqCst);
    let original: Binop = std::mem::transmute(ORIG.load(SeqCst));
    original(a, b).wrapping_mul(10)
}

unsafe extern "system" fn registry_detour_second(a: i32) -> i32 {
    a.wrapping_sub(1)
}

fn target_addr() -> usize {
    registry_target as Binop as usize
}

fn second_addr() -> usize {
    registry_second as Unop as usize
}

#[test]
fn hook_and_unhook_through_registry() {
    let _guard = lock();
    unsafe {
        assert_eq!(registry_target(9, 4), 5);

        let detour = registry_detour as Binop as usize;
        let tramp = nany::hook::detour::hook(target_addr(), detour).expect("hook");
        ORIG.store(tramp, SeqCst);

        assert_eq!(nany::hook::detour::trampoline(target_addr()), Some(tramp));
        assert!(nany::hook::detour::count() >= 1);
        assert_eq!(registry_target(9, 4), 50, "detour did not run");
        assert_eq!(HITS.load(SeqCst), 1);

        nany::hook::detour::unhook(target_addr());
        assert_eq!(registry_target(9, 4), 5, "unhook did not restore the target");
        assert_eq!(nany::hook::detour::trampoline(target_addr()), None);
        assert_eq!(nany::hook::detour::count(), 0);

        // removing twice is a no-op
        nany::hook::detour::unhook(target_addr());
        assert_eq!(nany::hook::detour::count(), 0);
    }
}

#[test]
fn unhook_all_clears_every_patch() {
    let _guard = lock();
    unsafe {
        assert_eq!(registry_second(3), 10);
        nany::hook::detour::hook(target_addr(), registry_detour as Binop as usize).expect("first hook");
        nany::hook::detour::hook(second_addr(), registry_detour_second as Unop as usize).expect("second hook");
        assert_eq!(registry_second(3), 2);
        assert_eq!(nany::hook::detour::count(), 2);

        assert_eq!(nany::hook::detour::unhook_all(), 2);
        assert_eq!(nany::hook::detour::count(), 0);
        assert_eq!(registry_second(3), 10, "unhook_all left a patch behind");
        assert_eq!(registry_target(9, 4), 5);
    }
}

#[test]
fn null_addresses_are_rejected() {
    let _guard = lock();
    unsafe {
        assert!(nany::hook::detour::hook(0, registry_detour as Binop as usize).is_none());
        assert!(nany::hook::detour::hook(target_addr(), 0).is_none());
        assert_eq!(nany::hook::detour::count(), 0);
        nany::hook::detour::unhook(0);
    }
}

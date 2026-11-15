//! Registry of where each live element sits, so a stale `UiRef` panics instead of reading freed memory.

use std::{
    collections::HashMap,
    sync::{
        LazyLock, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicU64, Ordering},
    },
};

use super::UiElement;

static NEXT_UID: AtomicU64 = AtomicU64::new(1);
static LIVE: LazyLock<Mutex<HashMap<u64, usize>>> = LazyLock::new(Default::default);

fn live() -> MutexGuard<'static, HashMap<u64, usize>> {
    LIVE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Unique for the whole process and never reused, unlike element ids.
pub(crate) fn next_uid() -> u64 {
    NEXT_UID.fetch_add(1, Ordering::Relaxed)
}

/// Records the element's current address; call whenever it is placed or moved.
pub(crate) fn register(element: &UiElement) {
    live().insert(element.uid, element as *const UiElement as usize);
}

pub(crate) fn unregister(uid: u64) {
    live().remove(&uid);
}

#[track_caller]
pub(crate) fn check(uid: u64, ptr: *const UiElement) {
    let found = live().get(&uid).copied();
    match found {
        Some(addr) if addr == ptr as usize => {}
        Some(_) => panic!(
            "stale UiRef (uid {uid}): the element moved because a sibling was added or removed in its parent; keep its id and look it up with `ui.get_element_mut(id)`"
        ),
        None => panic!(
            "stale UiRef (uid {uid}): the element was removed from the tree or never placed in it"
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::ui::{Container, ElementBuilder, Font, Ui};

    fn ui() -> Ui {
        Ui::create(true, Rc::new(Font::empty()))
    }

    #[test]
    fn fresh_ref_is_fine() {
        let mut ui = ui();
        let root = ui.add_child_to_root(Container::default().wrap());
        let child = ui.add_child(Container::default().wrap(), root).unwrap();
        assert_eq!(child.parent.unwrap().id(), root.id());
    }

    #[test]
    #[should_panic(expected = "element moved")]
    fn ref_dangles_after_sibling_realloc() {
        let mut ui = ui();
        let root = ui.add_child_to_root(Container::default().wrap());
        let first = ui.add_child(Container::default().wrap(), root).unwrap();
        for _ in 0..64 {
            ui.add_child(Container::default().wrap(), root);
        }
        let _ = first.id();
    }

    #[test]
    #[should_panic(expected = "element moved")]
    fn ref_dangles_after_sibling_insert() {
        let mut ui = ui();
        let root = ui.add_child_to_root(Container::default().wrap());
        for _ in 0..4 {
            ui.add_child(Container::default().wrap(), root);
        }
        let last = ui.add_child(Container::default().wrap(), root).unwrap();
        ui.insert_child(Container::default().wrap(), root, 0);
        let _ = last.id();
    }

    #[test]
    #[should_panic(expected = "was removed")]
    fn ref_dangles_after_remove() {
        let mut ui = ui();
        let root = ui.add_child_to_root(Container::default().wrap());
        let child = ui.add_child(Container::default().wrap(), root).unwrap();
        ui.remove_element(child);
        let _ = child.id();
    }

    #[test]
    fn parent_ptrs_follow_moves() {
        let mut ui = ui();
        let root = ui.add_child_to_root(Container::default().wrap());
        let a = ui.add_child(Container::default().wrap(), root).unwrap();
        ui.add_child(Container::default().wrap(), a);
        let a_id = a.id();
        for _ in 0..64 {
            ui.add_child(Container::default().wrap(), root);
        }
        let a = ui.get_element(a_id).unwrap();
        let grandchild = &a.childs[0];
        assert_eq!(grandchild.parent.unwrap().id(), a_id);
    }
}

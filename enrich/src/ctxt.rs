/*
Log context plumbing.

There's a lot of implementation details in this module for capturing and hydrating log contexts.
There are a few things that drive its design:

- Futures-based scopes should be cheap to poll (they might get polled a lot)
- Getting properties from a context should be cheap. A single scope will have multiple logs recorded, or none
- Scopes aren't used extensively
- Scopes usually have a single property

Most of this module isn't directly exposed to users, they can't currently interact with contexts directly.
*/

use std::{cell::RefCell, mem, sync::Arc};

use crate::properties::Properties;

thread_local!(static SHARED_CTXT: RefCell<SharedCtxt> = RefCell::new(Default::default()));

/**
A log context.

Each context contains a reference back to its parent and a set of properties.
The parent is used to check whether the context is being re-hydrated in the same context it was created in.
It's ok of it wasn't, that just means we need to re-create it.
*/
#[derive(Clone, Default, Debug)]
pub(crate) struct Ctxt {
    parent: Option<Arc<Ctxt>>,
    properties: Properties,
}

/**
A local context owned by a logger.

These contexts are pushed and popped from a shared context in scopes.
While the logger is in a scope it won't contain its original context.
That'll be restored before the scope returns.
*/
#[derive(Debug)]
pub(crate) struct LocalCtxt {
    inner: LocalCtxtInner,
}

#[derive(Debug)]
struct LocalCtxtInner {
    swapped: bool,
    kind: CtxtKind,
}

#[derive(Debug)]
enum CtxtKind {
    /**
    An original, local context.

    The parent of this conext will be the one that it was originally created in.
    */
    Local { local: Arc<Ctxt> },
    /**
    A joined context.

    If a context is sent to another thread, it might be hydrated with a different parent.
    That means we can't just clobber the shared context that's already on that thread, or properties might be lost.
    Instead we create a new context and cache it for the next time.
    */
    Joined {
        original: Arc<Ctxt>,
        joined: Arc<Ctxt>,
    },
    /**
    An empty context.

    This variant is only used by `SharedCtxt` as a starting point.
    If a `LocalCtxt` attempts to use an empty context it will probably panic (that's a bug).
    */
    Empty,
}

/**
A shared context.

Shared contexts are thread-local and make the currently scoped context available to loggers.
*/
#[derive(Debug)]
pub(crate) struct SharedCtxt {
    inner: CtxtKind,
}

impl Default for SharedCtxt {
    fn default() -> Self {
        SharedCtxt {
            inner: CtxtKind::Empty,
        }
    }
}

/**
A scope handle.

The handle allows a scoped closure to get a copy of the current context.
This context will be lazily fetched the first time it's asked for.
*/
pub(crate) struct Scope {
    ctxt: ScopeCtxt,
}

enum ScopeCtxt {
    Lazy,
    Loaded(Option<Arc<Ctxt>>),
}

impl ScopeCtxt {
    fn new() -> Self {
        ScopeCtxt::Lazy
    }

    fn get(&mut self) -> Option<&Arc<Ctxt>> {
        match *self {
            ScopeCtxt::Lazy => {
                let current = SHARED_CTXT.with(|shared| shared.borrow().current().cloned());

                *self = ScopeCtxt::Loaded(current);

                self.get()
            }
            ScopeCtxt::Loaded(ref ctxt) => ctxt.as_ref(),
        }
    }
}

impl Scope {
    fn new() -> Self {
        Scope {
            ctxt: ScopeCtxt::new(),
        }
    }

    pub(crate) fn current(&mut self) -> Option<&Ctxt> {
        self.ctxt.get().map(|ctxt| ctxt.as_ref())
    }
}

impl CtxtKind {
    fn is_empty(&self) -> bool {
        match *self {
            CtxtKind::Empty => true,
            _ => false,
        }
    }

    fn is_local(&self) -> bool {
        match *self {
            CtxtKind::Local { .. } => true,
            _ => false,
        }
    }

    fn is_joined(&self) -> bool {
        match *self {
            CtxtKind::Joined { .. } => true,
            _ => false,
        }
    }
}

impl LocalCtxt {
    pub(crate) fn new(ctxt: Arc<Ctxt>) -> Self {
        LocalCtxt {
            inner: LocalCtxtInner {
                kind: CtxtKind::Local { local: ctxt },
                swapped: false,
            },
        }
    }

    fn swap(&mut self, swap: &mut CtxtKind) {
        // Use the inner kind directly, because it may not be valid
        mem::swap(swap, self.raw_kind_mut());
        self.inner.swapped = !self.inner.swapped;
    }

    fn clear_joined(&mut self) {
        let kind = self.local_kind_mut();

        if kind.is_joined() {
            take_mut::take(kind, |ctxt| match ctxt {
                CtxtKind::Joined { original, .. } => CtxtKind::Local { local: original },
                ctxt => ctxt,
            })
        }
    }

    fn set_joined(&mut self, joined: Arc<Ctxt>) {
        let kind = self.local_kind_mut();

        if kind.is_local() {
            take_mut::take(kind, |ctxt| match ctxt {
                CtxtKind::Local { local } => CtxtKind::Joined {
                    original: local,
                    joined,
                },
                CtxtKind::Joined { original, .. } => CtxtKind::Joined { original, joined },
                ctxt => ctxt,
            });
        }
    }

    fn is_swapped(&self) -> bool {
        self.inner.swapped
    }

    fn is_empty(&self) -> bool {
        self.inner.kind.is_empty()
    }

    fn local_kind(&self) -> &CtxtKind {
        debug_assert!(!self.is_swapped(), "attempted to use swapped context");
        debug_assert!(!self.is_empty(), "attempted to use empty context");

        &self.inner.kind
    }

    fn local_kind_mut(&mut self) -> &mut CtxtKind {
        debug_assert!(!self.is_swapped(), "attempted to use swapped context");
        debug_assert!(!self.is_empty(), "attempted to use empty context");

        &mut self.inner.kind
    }

    fn raw_kind_mut(&mut self) -> &mut CtxtKind {
        &mut self.inner.kind
    }
}

impl SharedCtxt {
    fn current(&self) -> Option<&Arc<Ctxt>> {
        match self.inner {
            CtxtKind::Local { ref local } => Some(local),
            CtxtKind::Joined { ref joined, .. } => Some(joined),
            CtxtKind::Empty => None,
        }
    }

    pub(crate) fn scope_current<F, R>(f: F) -> R
    where
        F: FnOnce(&mut Scope) -> R,
    {
        f(&mut Scope::new())
    }

    pub(crate) fn scope<F, R>(local: &mut LocalCtxt, f: F) -> R
    where
        F: FnOnce(&mut Scope) -> R,
    {
        struct SharedGuard<'a> {
            shared: Option<&'a RefCell<SharedCtxt>>,
            local: Option<&'a mut LocalCtxt>,
        }

        impl<'a> SharedGuard<'a> {
            fn new(shared: &'a RefCell<SharedCtxt>, mut local: &'a mut LocalCtxt) -> Self {
                SharedCtxt::push(&mut shared.borrow_mut(), &mut local);
                SharedGuard {
                    shared: Some(&shared),
                    local: Some(local),
                }
            }
        }

        impl<'a> Drop for SharedGuard<'a> {
            fn drop(&mut self) {
                if let (Some(shared), Some(local)) = (self.shared.take(), self.local.take()) {
                    SharedCtxt::pop(&mut shared.borrow_mut(), local);
                }
            }
        }

        SHARED_CTXT.with(|shared| {
            let guard = SharedGuard::new(&shared, local);

            let ret = f(&mut Scope::new());

            drop(guard);

            ret
        })
    }

    fn swap_into_self(&mut self, local: &mut LocalCtxt) {
        debug_assert!(
            !local.is_swapped(),
            "the local context has already been swapped"
        );

        local.swap(&mut self.inner);
    }

    fn swap_out_of_self(&mut self, local: &mut LocalCtxt) {
        debug_assert!(local.is_swapped(), "the local context hasn't been swapped");

        local.swap(&mut self.inner);
    }

    fn push(shared: &mut SharedCtxt, incoming: &mut LocalCtxt) {
        // Check whether there's already an active context
        if let Some(shared_ctxt) = shared.current() {
            // If we have a joined context, check it first
            // If the shared context is invalid, then we might recreate it
            if let CtxtKind::Joined { ref joined, .. } = *incoming.local_kind() {
                if let Some(ref parent) = joined.parent {
                    if Arc::ptr_eq(shared_ctxt, parent) {
                        shared.swap_into_self(incoming);
                        return;
                    }
                }

                incoming.clear_joined();
            }

            // Check the parent of the original context
            if let CtxtKind::Local { ref local, .. } = *incoming.local_kind() {
                if let Some(ref parent) = local.parent {
                    if Arc::ptr_eq(shared_ctxt, parent) {
                        shared.swap_into_self(incoming);
                        return;
                    }
                }

                // If the original context isn't a child of the current one, create
                // a new joined context that combines them.
                let joined = Arc::new(Ctxt::from_shared(
                    local.properties.clone(),
                    Some(shared_ctxt.clone()),
                ));
                incoming.set_joined(joined);

                shared.swap_into_self(incoming);
                return;
            }

            panic!("incoming context `{:?}` is invalid", incoming);
        } else {
            // Make sure the joined context is `None`
            // If this context is the root of this thread then there's no need for it
            incoming.clear_joined();

            let mut root_ctxt = SharedCtxt {
                inner: CtxtKind::Empty,
            };

            root_ctxt.swap_into_self(incoming);

            *shared = root_ctxt;
        }
    }

    fn pop(shared: &mut SharedCtxt, outgoing: &mut LocalCtxt) {
        shared.swap_out_of_self(outgoing);
    }
}

impl Ctxt {
    /// Create a local context from a set of properties and a shared context.
    ///
    /// If the shared context is `Some`, then the local context will contain the union
    /// of `properties` and the properties on the shared context.
    fn from_shared(mut properties: Properties, shared: Option<Arc<Ctxt>>) -> Self {
        properties.extend(
            shared
                .as_ref()
                .map(|shared| &shared.properties)
                .unwrap_or(&Properties::Empty),
        );

        Ctxt {
            parent: shared,
            properties,
        }
    }

    pub(crate) fn from_scope(properties: Properties, scope: &mut Scope) -> Self {
        Ctxt::from_shared(properties, scope.ctxt.get().cloned())
    }

    pub(crate) fn properties(&self) -> &Properties {
        &self.properties
    }
}

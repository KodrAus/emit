use core::{fmt, ops::ControlFlow};

use crate::{
    extent::{Extent, ToExtent},
    props::{ByRef, ErasedProps, Props},
    template::{Render, Template},
};

#[derive(Clone)]
pub struct Event<'a, P> {
    extent: Option<Extent>,
    tpl: Template<'a>,
    props: P,
}

impl<'a, P> Event<'a, P> {
    pub fn new(extent: impl ToExtent, tpl: Template<'a>, props: P) -> Self {
        Event {
            extent: extent.to_extent(),
            tpl,
            props,
        }
    }

    pub fn extent(&self) -> Option<&Extent> {
        self.extent.as_ref()
    }

    pub fn tpl(&self) -> Template {
        self.tpl.by_ref()
    }

    pub fn props(&self) -> &P {
        &self.props
    }
}

impl<'a, P: Props> Event<'a, P> {
    pub fn msg(&self) -> Render<&P> {
        self.tpl.render(&self.props)
    }

    pub fn by_ref<'b>(&'b self) -> Event<'b, ByRef<'b, P>> {
        Event {
            extent: self.extent.clone(),
            tpl: self.tpl.by_ref(),
            props: self.props.by_ref(),
        }
    }

    pub fn erase<'b>(&'b self) -> Event<'b, &'b dyn ErasedProps> {
        Event {
            extent: self.extent.clone(),
            tpl: self.tpl.by_ref(),
            props: &self.props,
        }
    }
}

impl<'a, P: Props> fmt::Debug for Event<'a, P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct AsDebug<T>(T);

        impl<T: Props> fmt::Debug for AsDebug<T> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                let mut f = f.debug_struct("");

                self.0.for_each(|k, v| {
                    f.field(k.as_str(), &v);

                    ControlFlow::Continue(())
                });

                f.finish()
            }
        }

        let mut f = f.debug_struct("Event");

        f.field("extent", &self.extent);
        f.field("msg", &self.msg());
        f.field("tpl", &self.tpl);
        f.field("props", &AsDebug(&self.props));

        f.finish()
    }
}

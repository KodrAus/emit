#[derive(Clone)]
pub struct Template<'a>(fv_template::rt::Template<'a>);

impl<'a> Template<'a> {
    pub fn by_ref<'b>(&'b self) -> Template<'b> {
        todo!()
    }
}

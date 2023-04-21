pub struct Tpl<'a>(fv_template::rt::Template<'a>);

impl<'a> Tpl<'a> {
    pub fn by_ref<'b>(&'b self) -> Tpl<'b> {
        todo!()
    }
}

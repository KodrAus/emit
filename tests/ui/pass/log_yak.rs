#![feature(stmt_expr_attributes, proc_macro_hygiene)]

#[derive(Debug)]
pub struct Yak(String);

impl Yak {
    fn shave(&mut self, _: u32) {}
}

fn find_a_razor() -> Result<u32, std::io::Error> {
    Ok(1)
}

pub fn shave_the_yak(yak: &mut Yak) {
    emit::info!("Commencing yak shaving for {#[emit::as_debug] yak}");

    loop {
        match find_a_razor() {
            Ok(razor) => {
                emit::info!("Razor located: {razor}");
                yak.shave(razor);
                break;
            }
            Err(err) => {
                emit::warn!("Unable to locate a razor: {err}, retrying");
            }
        }
    }
}

fn main() {
    tracing_subscriber::fmt().init();
    
    shave_the_yak(&mut Yak("🐮".to_owned()));
}

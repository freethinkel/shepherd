fn main() {
    let rt = tokio::runtime::Runtime::new().expect("tokio");
    let handle = rt.handle().clone();
    gpui_platform::application().run(move |cx| {
        shepherd::shell::Shell::open(handle.clone(), cx);
        cx.activate(true);
    });
    drop(rt);
}

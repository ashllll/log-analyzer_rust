fn main() {
    // Windows：将应用程序清单嵌入可执行文件
    // 声明 longPathAware=true，使系统注册表长路径设置对本应用生效
    #[cfg(target_os = "windows")]
    embed_windows_manifest();

    tauri_build::build()
}

#[cfg(target_os = "windows")]
fn embed_windows_manifest() {
    let mut res = winres::WindowsResource::new();
    res.set_manifest(include_str!("windows-app.manifest"));
    res.compile().expect("嵌入 Windows 应用程序清单失败");
}

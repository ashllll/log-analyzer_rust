fn main() {
    // Windows：通过 tauri-build 原生 API 嵌入长路径感知 manifest（longPathAware=true）
    // 无需外部 winres 依赖，避免 CI 上 rc.exe 路径探测失败问题
    let attrs = tauri_build::Attributes::new();

    #[cfg(target_os = "windows")]
    let attrs = attrs.windows_attributes(
        tauri_build::WindowsAttributes::new().app_manifest(include_str!("windows-app.manifest")),
    );

    tauri_build::try_build(attrs).expect("tauri-build 失败");
}

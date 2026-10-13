fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/em624.ico");
        res.set(
            "FileDescription",
            "E-DRA EM624 Mouse Configurator (Reversed)",
        );
        res.set("ProductName", "E-DRA EM624 Reversed Driver");
        res.set("OriginalFilename", "em624-reversed.exe");
        res.set("InternalName", "em624-reversed");
        let _ = res.compile();
    }
}

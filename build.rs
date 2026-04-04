fn main() {
    #[cfg(target_os = "windows")]
    {
        let rc_file = std::path::Path::new("resources/windows/trawls.rc");
        println!("cargo:rerun-if-changed={}", rc_file.display());
        embed_resource::compile(rc_file, embed_resource::NONE)
            .manifest_required()
            .unwrap();
    }
}

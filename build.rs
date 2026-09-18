fn main() {
    embed_resource::compile("tts-tool.rc", embed_resource::NONE)
        .manifest_required()
        .expect("无法嵌入 Windows 应用程序清单");
    println!("cargo:rerun-if-changed=tts-tool.rc");
    println!("cargo:rerun-if-changed=app.manifest");
}

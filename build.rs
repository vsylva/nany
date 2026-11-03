use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=src/render/vk_shader.wgsl");

    let src = match std::fs::read_to_string("src/render/vk_shader.wgsl",)
    {
        Ok(s,) => s,
        Err(e,) =>
        {
            println!("cargo:warning=读不到 vk_shader.wgsl: {e}");
            return;
        }
    };
    let module = match naga::front::wgsl::parse_str(&src,)
    {
        Ok(m,) => m,
        Err(e,) => panic!("WGSL 解析失败: {e:?}"),
    };
    let info =
        match naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::empty(),)
            .validate(&module,)
        {
            Ok(i,) => i,
            Err(e,) => panic!("WGSL 校验失败: {e:?}"),
        };
    let options = naga::back::spv::Options { lang_version: (1, 0,), ..Default::default() };
    let words = match naga::back::spv::write_vec(&module, &info, &options, None,)
    {
        Ok(w,) => w,
        Err(e,) => panic!("SPIR-V 生成失败: {e:?}"),
    };

    let mut out = String::with_capacity(words.len() * 12,);
    out.push_str("// 由 build.rs 从 vk_shader.wgsl 生成（naga → SPIR-V），不要手改\n",);
    out.push_str(&format!("pub const VK_SPV_WORDS: usize = {};\n", words.len()),);
    out.push_str("pub const VK_SPV: &[u32] = &[\n",);
    for (i, w,) in words.iter().enumerate()
    {
        if i % 8 == 0
        {
            out.push_str("    ",);
        }
        out.push_str(&format!("{w:#010x}, "),);
        if i % 8 == 7
        {
            out.push('\n',);
        }
    }
    out.push_str("\n];\n",);

    let dir = PathBuf::from(std::env::var("OUT_DIR",).expect("OUT_DIR",),);
    std::fs::write(dir.join("vk_spv.rs",), out,).expect("写 vk_spv.rs",);
}

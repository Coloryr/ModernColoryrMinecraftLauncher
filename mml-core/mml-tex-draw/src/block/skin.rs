//! 玩家皮肤方块：把皮肤文件渲染成头颅图标，作为自定义方块注册进block.json
//!
//! ID = `custom:<名字>`（命名空间与原版方块隔离，绝不冲突），分组固定为[`SKIN_CAT`]
//! （独立于原版itemGroup分组）。同名重复添加即覆盖（重渲染并替换图标）；
//! [`remove_skin_block`]删除时连同图标文件一起删。注册只往block.json的
//! tex/cat表里插条目，全量重渲染（render_blocks）只做插入不清表，自定义条目能跨版本保留

use std::collections::HashMap;

use mml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType, SkinBlockErrorData};
use mml_sys::path_helper;

use crate::gpu::GpuCtx;

/// 皮肤方块在block.json里的分组值（itemGroup lang键尾段同一套的独立分组）
pub const SKIN_CAT: &str = "playerSkin";

/// 皮肤方块的ID前缀（ID = 前缀 + 名字，图标文件名同样把':'换成'_'）
const ID_PREFIX: &str = "custom:";

/// 校验名字并拼出方块ID（只允许英文字母数字-_，≤64字符，防路径注入）
fn skin_id(name: &str) -> CoreResult<String> {
    let ok = !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'));
    if !ok {
        return Err(ErrorType::SkinBlockError(SkinBlockErrorData::NameIllegal(
            name.to_string(),
        )));
    }
    Ok(format!("{ID_PREFIX}{name}"))
}

/// 添加（或覆盖）一个玩家皮肤方块：渲染头颅图标 → 写盘 → 注册进block.json
///
/// `name`：自定义名（只允许英文字母数字-_，显示名即它）；
/// `skin_png`：皮肤PNG数据，64×64（带帽子层）或旧版64×32（单层）。
/// 返回方块ID（custom:<名字>）；同名时覆盖旧图标与注册信息
pub fn add_skin_block(name: &str, skin_png: &[u8]) -> CoreResult<String> {
    let id = skin_id(name)?;
    let out_name = format!("{}.png", id.replace(':', "_"));

    // 解码并校验皮肤尺寸（slim/wide只差手臂，头部一致，无需区分）
    let tex = super::decode_png(skin_png)
        .ok_or(ErrorType::SkinBlockError(SkinBlockErrorData::DecodeFail))?;
    let hat = match (tex.width(), tex.height()) {
        (64, 64) => true,
        (64, 32) => false,
        (w, h) => {
            return Err(ErrorType::SkinBlockError(SkinBlockErrorData::SkinSize {
                width: w,
                height: h,
            }))
        }
    };

    // 渲染头颅图标（带帽子层；贴图key用任意固定值，与quad里的引用对应即可）
    let Some(gpu) = GpuCtx::try_new() else {
        return Err(ErrorType::GpuNotAvailable);
    };
    let model = super::bake_head_model(tex.clone(), hat, "skin");
    let textures = HashMap::from([("skin".to_string(), tex)]);
    let rgba = gpu
        .render(&model, &textures, None, super::BLOCK_SIZE as u32)
        .ok_or(ErrorType::SkinBlockError(SkinBlockErrorData::RenderFail))?;

    let mut buf = Vec::new();
    {
        let mut encoder = png::Encoder::new(
            &mut buf,
            super::BLOCK_SIZE as u32,
            super::BLOCK_SIZE as u32,
        );
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .and_then(|mut w| w.write_image_data(&rgba))
            .map_err(|e| ErrorType::TaskError(ErrorData { error: e.to_string() }))?;
    }

    let dir = crate::get_block_dir().ok_or(ErrorType::DownloadFileFail)?;
    path_helper::write_bytes(dir.join(&out_name), &buf)?;

    // 注册进方块表（name不写：GUI翻译miss时回退ID尾段，正好就是自定义名）
    {
        let mut blocks = crate::blocks_write();
        blocks.tex.insert(id.clone(), out_name);
        blocks.cat.insert(id.clone(), SKIN_CAT.to_string());
    }
    crate::save()?;

    Ok(id)
}

/// 删除一个皮肤方块：移出block.json并删除图标文件
///
/// 只允许删皮肤分组（playerSkin）的条目，原版方块删不掉
pub fn remove_skin_block(name: &str) -> CoreResult<()> {
    let id = skin_id(name)?;

    let out_name = {
        let mut blocks = crate::blocks_write();
        if blocks.cat.get(&id).map(|c| c.as_str()) != Some(SKIN_CAT) {
            return Err(ErrorType::SkinBlockError(SkinBlockErrorData::NotFound(id)));
        }
        let out_name = blocks.tex.remove(&id);
        blocks.cat.remove(&id);
        out_name
    };

    if let (Some(dir), Some(out_name)) = (crate::get_block_dir(), out_name) {
        let _ = path_helper::delete(dir.join(out_name));
    }
    crate::save()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 名字校验：合法字符放行，空名/路径注入/超长拒绝
    #[test]
    fn skin_id_validation() {
        assert_eq!(skin_id("steve").unwrap(), "custom:steve");
        assert!(skin_id("A-b_9").is_ok());
        assert!(skin_id(&"x".repeat(64)).is_ok());

        assert!(skin_id("").is_err());
        assert!(skin_id("../evil").is_err());
        assert!(skin_id("a b").is_err());
        assert!(skin_id("minecraft:stone").is_err());
        assert!(skin_id(&"x".repeat(65)).is_err());
    }

    /// 完整流程：添加（出图+注册）→ 同名覆盖 → 删除（注销+删文件）
    ///
    /// 走临时目录+真实GPU渲染，且init路径是进程级OnceLock，单独跑：
    /// cargo test -p mml-tex-draw --lib skin::tests::add_remove_roundtrip -- --ignored
    #[test]
    #[ignore]
    fn add_remove_roundtrip() {
        let root = crate::unique_temp_dir("skin");
        crate::init(&root).unwrap();

        let skin_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../mml-skin-draw/tests/skin_slim.png");
        let skin = std::fs::read(&skin_path).unwrap();

        // 添加：出图 + 注册进独立分组
        let id = add_skin_block("test_skin", &skin).unwrap();
        assert_eq!(id, "custom:test_skin");
        let png = crate::get_block_path(&id).unwrap();
        assert!(png.exists(), "图标未写盘：{}", png.display());
        assert_eq!(crate::block_cat(&id).unwrap(), SKIN_CAT);
        assert!(crate::blocks().iter().any(|b| b == &id), "未注册进方块列表");

        // 同名覆盖：不报错，图标仍在
        add_skin_block("test_skin", &skin).unwrap();
        assert!(crate::get_block_path(&id).unwrap().exists());

        // 删除：注销 + 删文件；再删报不存在
        remove_skin_block("test_skin").unwrap();
        assert!(crate::get_block_path(&id).is_none());
        assert!(!png.exists(), "图标文件未删除");
        assert!(remove_skin_block("test_skin").is_err());

        let _ = std::fs::remove_dir_all(&root);
    }
}

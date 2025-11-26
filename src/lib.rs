//! kovi-plugin-image-splitter
//!
//! 图片九宫格/网格裁剪插件
//!
//! 功能：
//! 1. 引用一张图片或发送图片时附带指令。
//! 2. 指令格式：`裁剪 行x列` (例如：裁剪 3*3, 切图 2x2)。
//! 3. 机器人会将图片切割成指定份数，并通过合并转发发送。

// =============================
//          Modules
// =============================

mod utils {
    use kovi::MsgEvent;
    use regex::Regex;
    use std::sync::OnceLock;

    pub static CMD_REGEX: OnceLock<Regex> = OnceLock::new();

    /// 解析指令，返回 (行, 列)
    pub fn parse_command(text: &str) -> Option<(u32, u32)> {
        // 匹配模式： "裁剪" 或 "切图" + 空格(可选) + 数字 + 分隔符(*, x, X, 空格) + 数字
        let re = CMD_REGEX.get_or_init(|| {
            Regex::new(r"^(?:裁剪|切图|分割)\s*(\d+)\s*(?:[\*xX× ])\s*(\d+)$").unwrap()
        });

        re.captures(text.trim()).map(|caps| {
            let rows = caps.get(1).unwrap().as_str().parse::<u32>().unwrap_or(3);
            let cols = caps.get(2).unwrap().as_str().parse::<u32>().unwrap_or(3);
            (rows, cols)
        })
    }

    /// 从消息或引用中获取图片 URL
    pub async fn get_image_url(
        event: &std::sync::Arc<MsgEvent>,
        bot: &std::sync::Arc<kovi::RuntimeBot>,
    ) -> Option<String> {
        // 1. 优先检查当前消息中是否有图片
        for seg in event.message.iter() {
            if seg.type_ == "image"
                && let Some(url) = seg.data.get("url").and_then(|u| u.as_str())
            {
                return Some(url.to_string());
            }
        }

        // 2. 检查引用消息
        let reply_id = event.message.iter().find_map(|seg| {
            if seg.type_ == "reply" {
                seg.data.get("id").and_then(|v| v.as_str())
            } else {
                None
            }
        })?;

        // 获取原消息
        if let Ok(reply_id_int) = reply_id.parse::<i32>()
            && let Ok(msg_res) = bot.get_msg(reply_id_int).await
            && let Some(segments) = msg_res.data.get("message").and_then(|v| v.as_array())
        {
            for seg in segments {
                if let Some(type_) = seg.get("type").and_then(|t| t.as_str())
                    && type_ == "image"
                    && let Some(url) = seg
                        .get("data")
                        .and_then(|d| d.get("url"))
                        .and_then(|u| u.as_str())
                {
                    return Some(url.to_string());
                }
            }
        }

        None
    }
}

mod splitter {
    use anyhow::{Context, Result};
    use base64::{Engine as _, engine::general_purpose};
    use image::GenericImageView;
    use std::io::Cursor;

    /// 下载图片
    pub async fn download_image(url: &str) -> Result<bytes::Bytes> {
        let resp = reqwest::get(url).await?;
        let bytes = resp.bytes().await?;
        Ok(bytes)
    }

    /// 核心逻辑：裁剪图片
    /// 返回 Base64 字符串列表
    pub fn split_image_blocking(
        img_bytes: bytes::Bytes,
        rows: u32,
        cols: u32,
    ) -> Result<Vec<String>> {
        // 限制最大行列，防止滥用导致 OOM
        if rows > 10 || cols > 10 {
            return Err(anyhow::anyhow!("切片数量过多，最大支持 10x10"));
        }
        if rows == 0 || cols == 0 {
            return Err(anyhow::anyhow!("行或列不能为 0"));
        }

        // 加载图片
        let img =
            image::load_from_memory(&img_bytes).context("Failed to load image from memory")?;

        let (width, height) = img.dimensions();
        let tile_width = width / cols;
        let tile_height = height / rows;

        if tile_width == 0 || tile_height == 0 {
            return Err(anyhow::anyhow!("图片太小，无法按照指定规格裁剪"));
        }

        let mut base64_list = Vec::with_capacity((rows * cols) as usize);

        // 遍历裁剪
        for r in 0..rows {
            for c in 0..cols {
                let x = c * tile_width;
                let y = r * tile_height;

                // crop_imm 是不可变裁剪，开销较小
                let sub_img = img.view(x, y, tile_width, tile_height).to_image();

                // 编码回图片格式 (PNG 以保留透明度)
                let mut buffer = Cursor::new(Vec::new());
                sub_img
                    .write_to(&mut buffer, image::ImageFormat::Png)
                    .context("Failed to encode sub-image")?;

                let b64 = general_purpose::STANDARD.encode(buffer.get_ref());
                base64_list.push(b64);
            }
        }

        Ok(base64_list)
    }
}

// =============================
//      Main Plugin Logic
// =============================

use kovi::bot::message::Segment;
use kovi::serde_json::json;
use kovi::{PluginBuilder, log};
use kovi_plugin_expand_napcat::NapCatApi;

#[kovi::plugin]
async fn main() {
    let bot = PluginBuilder::get_runtime_bot();

    PluginBuilder::on_msg(move |event| {
        let bot = bot.clone();
        async move {
            let text = match event.borrow_text() {
                Some(t) => t,
                None => return,
            };

            // 1. 解析指令
            if let Some((rows, cols)) = utils::parse_command(text) {
                // 2. 获取图片 URL
                let url = match utils::get_image_url(&event, &bot).await {
                    Some(u) => u,
                    None => {
                        // event.reply("⚠️ 请在发送指令时附带图片，或引用一张图片");
                        return;
                    }
                };

                event.reply(format!(
                    "✂️ 正在将图片裁剪为 {}行 x {}列，请稍候...",
                    rows, cols
                ));

                // 3. 下载图片
                let img_bytes = match splitter::download_image(&url).await {
                    Ok(b) => b,
                    Err(e) => {
                        log::error!("Download failed: {}", e);
                        event.reply("❌ 图片下载失败");
                        return;
                    }
                };

                // 4. 在阻塞线程中处理图片 (CPU 密集型)
                let split_task = kovi::tokio::task::spawn_blocking(move || {
                    splitter::split_image_blocking(img_bytes, rows, cols)
                });

                match split_task.await {
                    Ok(Ok(base64_list)) => {
                        // 5. 构建合并转发消息
                        let mut nodes = Vec::new();
                        let bot_info_res = bot.get_login_info().await;
                        let bot_info = match bot_info_res {
                            Ok(info) => info,
                            Err(_) => {
                                event.reply("❌ 获取机器人信息失败");
                                return;
                            }
                        };
                        let bot_id = bot_info
                            .data
                            .get("user_id")
                            .and_then(|u| u.as_str())
                            .unwrap_or("0");
                        let bot_name = bot_info
                            .data
                            .get("nickname")
                            .and_then(|n| n.as_str())
                            .unwrap_or("Bot");

                        for b64 in base64_list {
                            let node = Segment::new(
                                "node",
                                json!({
                                    "name": bot_name,
                                    "uin": bot_id,
                                    "content": [
                                        {
                                            "type": "image",
                                            "data": {
                                                "file": format!("base64://{}", b64)
                                            }
                                        }
                                    ]
                                }),
                            );
                            nodes.push(node);
                        }

                        // 发送合并转发
                        if let Some(group_id) = event.group_id {
                            let _ = bot.send_group_forward_msg(group_id, nodes).await;
                        } else {
                            let user_id_i64 = event.user_id;
                            if user_id_i64 != 0 {
                                let _ = bot.send_private_forward_msg(user_id_i64, nodes).await;
                            } else {
                                // 退化为直接发图
                                let mut msg = kovi::Message::new();
                                for node in nodes {
                                    if let Some(content) =
                                        node.data.get("content").and_then(|c| c.as_array())
                                        && let Some(file) = content[0]
                                            .get("data")
                                            .and_then(|d| d.get("file"))
                                            .and_then(|f| f.as_str())
                                    {
                                        msg = msg.add_image(file);
                                    }
                                }
                                event.reply(msg);
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        event.reply(format!("❌ 图片处理失败: {}", e));
                    }
                    Err(e) => {
                        log::error!("Task join error: {}", e);
                        event.reply("❌ 系统内部错误");
                    }
                }
            }
        }
    });
}

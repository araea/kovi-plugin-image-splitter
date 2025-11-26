kovi-plugin-image-splitter
==========================

[<img alt="github" src="https://img.shields.io/badge/github-araea/kovi__plugin__image__splitter-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/araea/kovi-plugin-image-splitter)
[<img alt="crates.io" src="https://img.shields.io/crates/v/kovi-plugin-image-splitter.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/kovi-plugin-image-splitter)

Kovi 的图片智能裁剪插件。支持自定义网格（九宫格、六宫格等），支持通过合并转发发送切片，防止刷屏。

## 特性

- ✂️ **灵活指令** - 支持 `3*3`、`2x2`、`4 4` 等多种分隔符格式
- 🖼️ **智能识别** - 支持处理当前消息中的图片，或**引用回复**中的图片
- 📦 **合并转发** - 自动将裁剪后的碎片打包发送，保持群聊版面整洁
- ⚡ **高性能** - 异步下载，独立线程处理图片，不阻塞 Bot 响应

## 前置

1. 创建 Kovi 项目
2. 执行 `cargo kovi add image-splitter`
3. 在 `src/main.rs` 中添加 `kovi_plugin_image_splitter`

## 快速开始

1. 发送一张图片，并附带文字 `裁剪 3*3`
2. 或者，引用一张别人发送的图片，发送 `切图 2x2`
3. 机器人将回复裁剪好的图片集合

## 指令详解

插件使用正则匹配，支持多种分隔符（`*`, `x`, `X`, `×`, `空格`）。

### 基础语法

`指令` + `行数` + `分隔符` + `列数`

| 示例 | 效果 | 说明 |
|------|------|------|
| `裁剪 3*3` | 3行3列 (9张) | 标准九宫格 |
| `切图 2x2` | 2行2列 (4张) | 四宫格 |
| `分割 6 4` | 6行4列 (24张) | 高密度网格 |
| `裁剪 1x3` | 1行3列 (3张) | 横向长图切分 |

### 触发方式

**方式一：图文同发**
> [图片]
> 裁剪 3*3

**方式二：引用图片**
> [引用某人的图片]
> 切图 2x2

## 注意事项

- 为了防止恶意消耗服务器资源，最大裁剪限制为 **10x10**。

## 致谢

- [Kovi](https://kovi.threkork.com/)
- [image-rs](https://github.com/image-rs/image)

<br>

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>

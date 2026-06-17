# imageoptim-rs

[![Release](https://img.shields.io/github/v/release/liut/imageoptim-rs)](https://github.com/liut/imageoptim-rs/releases/latest)
[![CI](https://github.com/liut/imageoptim-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/liut/imageoptim-rs/actions/workflows/ci.yml)
[![License: GPL v3+](https://img.shields.io/badge/License-GPLv3+-blue.svg)](LICENSE)

由原生 Rust crate 提供支持的跨平台图片优化命令行工具。

灵感来自 [`JamieMason/ImageOptim-CLI`](https://github.com/JamieMason/ImageOptim-CLI)（3.5k stars，2023-11 起归档）。但与原项目不同——原项目是仅限 macOS 的 AppleScript 编排器，通过驱动 GUI 应用程序来工作——`imageoptim-rs` 直接使用 Rust crate。它可在 macOS、Linux 和 Windows 上运行，除了 C 标准库外没有任何运行时依赖，并以单一静态二进制形式发布。每个 [GitHub Release](https://github.com/liut/imageoptim-rs/releases) 都附带了 `x86_64-unknown-linux-gnu`、`x86_64-apple-darwin`、`aarch64-apple-darwin` 和 `x86_64-pc-windows-msvc` 的预编译二进制。

> **许可证声明**。`imageoptim-rs` 采用 **GPL-3.0-or-later** 许可证。这是一种著佐权（copyleft）许可证：你分发的任何链接了本代码的二进制，也必须采用 GPL-3.0-or-later，并且必须提供源代码。采用 GPL 是因为可选的 PNG 有损压缩路径链接了 [libimagequant](https://github.com/ImageOptim/libimagequant)，而后者的许可证就是 GPL。如果你无法接受 GPL 条款，请勿使用本二进制。

## 安装

### 预编译二进制（推荐）

从 [latest release](https://github.com/liut/imageoptim-rs/releases/latest) 下载适用于你平台的归档文件并解压。归档里只有一个 `imageoptim`（Windows 上是 `imageoptim.exe`）可执行文件——无需安装器、无运行时依赖，放到 `$PATH` 的任何位置即可。

```bash
# Linux x86_64
curl -L https://github.com/liut/imageoptim-rs/releases/latest/download/imageoptim-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo install -b imageoptim /usr/local/bin/

# macOS（Apple Silicon）
curl -L https://github.com/liut/imageoptim-rs/releases/latest/download/imageoptim-aarch64-apple-darwin.tar.gz | tar xz
sudo install -b imageoptim /usr/local/bin/

# Windows（PowerShell 5.1+）
$ErrorActionPreference = 'Stop'
$installDir = "$env:LOCALAPPDATA\Programs\imageoptim"
$zipPath = Join-Path $env:TEMP 'imageoptim.zip'

Invoke-WebRequest -Uri 'https://github.com/liut/imageoptim-rs/releases/latest/download/imageoptim-x86_64-pc-windows-msvc.zip' -OutFile $zipPath -UseBasicParsing

New-Item -ItemType Directory -Force -Path $installDir | Out-Null
Expand-Archive -Path $zipPath -DestinationPath $installDir -Force

# 把安装目录加入用户 PATH（对新 shell 持久生效）
$currentUserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($currentUserPath -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable('Path', "$currentUserPath;$installDir", 'User')
}
# 同时更新当前 shell 的 PATH
$env:Path = "$env:Path;$installDir"

# 验证安装
imageoptim --version
```

### 从源码编译

```bash
git clone https://github.com/liut/imageoptim-rs
cd imageoptim-rs
cargo build --release
./target/release/imageoptim --help
```

`cargo install --path .` 也可用，会把构建版本锁定到你当前 checkout 的源码树。

## 快速上手

就地优化当前目录下的所有 PNG：

```bash
imageoptim '*.png'
```

递归处理子目录中的所有 JPEG：

```bash
imageoptim '**/*.jpg' -r
```

预览会发生什么，但不实际修改任何文件：

```bash
imageoptim 'assets/**/*.png' -r --dry-run
```

使用 4 个并行 worker：

```bash
imageoptim '**/*.{png,jpg,gif,webp,svg}' -j 4
```

## 支持的格式

| 格式 | 优化器 | 说明 |
| --- | --- | --- |
| PNG | `oxipng`（无损）或 `imagequant`（`--lossy` 启用有损） | 默认无损；`--lossy` 启用最多 256 色的调色板量化，可通过 `--max-colors` 调节；oxipng 预设可通过 `--png-optimization-level` 调节 |
| JPEG | `jpeg-decoder` + `jpeg-encoder` | 有损重编码，默认质量 85；输入中的 EXIF/IPTC/XMP/ICC/注释标记会被隐式丢弃（编码器从原始像素重新编码，只输出必需的 JFIF APP0 头） |
| GIF | `gif` crate | 用 NeuQuant 索引重编码（`--quality` 被忽略） |
| WebP | `webp` + `image` | 当 `--quality` 被设置时为有损重编码，否则为无损 |
| SVG | `usvg` | 规范化重序列化；不是完整的 minifier（`--quality` 被忽略） |

`--quality <0-100>` 控制 JPEG 和 WebP 的有损质量。GIF 和 SVG 始终是无损的，该参数会被静默忽略。

`--lossy` 启用 PNG 的调色板量化。这是驱动 [pngquant](https://pngquant.org/) 和 ImageOptim.app 中 "Lossy" 复选框的同一套算法：每个像素的颜色被映射到最多 256 色调色板中最近的一项，可将照片类 PNG 缩小 50–80%，代价是出现细微的色带。输出仍是合法 PNG，但不再与原文件逐字节相同——首次用于真实素材时请保留 `.bak`（或使用 `--dry-run`）。可用 `--max-colors <N>`（2..=256，需配合 `--lossy`）来限制调色板大小；更小的 N 意味着更明显的色带但更小的文件。

## 选项

```
Usage: imageoptim [OPTIONS] [PATTERN]...

Arguments:
  [PATTERN]...  文件路径或 glob 模式（如 `*.png`、`assets/**/*.jpg`）

Options:
  -r, --recursive        递归处理子目录
      --dry-run          展示将要做什么但不实际修改任何文件
      --no-color         关闭 ANSI 颜色输出
      --no-backup        跳过在覆盖前创建 `<path>.bak`
      --lossy            允许 PNG 调色板量化（默认关闭）
      --max-colors <N>   把有损模式的调色板大小限制为 N（2-256，需配合 --lossy）
      --no-zopfli        跳过 `--lossy` 路径中可选的 `zopflipng` 后置步骤
      --png-optimization-level <0-6>  覆盖 oxipng 预设（默认 无损 3、有损 6）
      --output-dir <DIR> 把优化后的文件输出到 `<DIR>/<stem>_s<ext>` 而不是就地覆盖
      --fail-fast        在第一个文件出错时立即停止处理
  -q, --quality <0-100>  有损格式的质量（0-100）。省略时为无损
  -j, --jobs <N>         并行 worker 数量
  -v, --verbose          把每一步优化的细节输出到 stderr
      --summary-only     抑制逐文件结果行；只打印汇总
  -h, --help             打印帮助
  -V, --version          打印版本
```

## 安全契约

`imageoptim-rs` 只在以下**两个**条件都满足时才会覆盖一个文件：

1. 优化输出严格小于原始文件。
2. 优化输出能被解码回同一种格式的有效图像。

如果任一条件失败——例如文件已经是最优压缩、或编码器产出了畸形的结果——该文件保持不动，并被报告为 `skipped`。

优化失败的文件（例如 oxipng 对损坏 PNG 出错）不会覆盖原文件；处理会继续处理其他文件。如果任何文件失败，进程以退出码 1 退出。

### 进度条

当 stderr 连接到终端时，`imageoptim-rs` 会在处理过程中绘制进度条。在以下情况下会自动隐藏：

- stdout/stderr 被重定向（例如被管道到文件或其他命令），以保持日志干净
- 设置了 `--dry-run`，因为没有等待的事情

### 可选的 `zopflipng` 后置步骤

`--lossy` PNG 流水线运行三步：

1. **pngquant** 把图像量化为 256 色调色板（内嵌的 libimagequant）。
2. **oxipng** 以最大压缩重编码调色板 PNG，内置 zopfli-deflate 搜索的 `--iterations=12`。
3. **`zopflipng`**（若已安装）重新选择 PNG filter 并运行更深的 deflate 搜索——通常还能再压缩 10–20%。

当在 `$PATH` 中找到 `zopflipng` 时自动调用第 3 步。在没有 `zopflipng` 的系统上首次运行会向 stderr 打印一条提示，指向安装命令。传 `--no-zopfli` 可跳过该步（并隐藏提示）。

安装方式：

- macOS：`brew install zopfli`
- Debian / Ubuntu：`apt install zopfli`
- 从源码：<https://github.com/google/zopfli>

### 详细模式（`-v` / `--verbose`）

传 `-v` 会把每一步优化的细节输出到 stderr。对一次 PNG 运行，trace 形如：

```
imageoptim: png lossy → decoded 1122x1402 RGBA8 (1573044 pixels)
imageoptim:   imagequant q=80-100 max_colors=256 speed=3
imageoptim:   imagequant produced 256 entries in the palette
imageoptim:   oxipng preset 6 (oxipng's internal zopfli, 12 iterations)
imageoptim:   zopflipng not installed; skipped
  [PNG] tests/example01.png saved 2.30 MB (80.61%)
```

trace 能区分 "zopflipng not installed" 与 "zopflipng installed but failed"，方便一眼判断是否需要安装 `zopfli`。其他格式（JPEG、GIF、WebP、SVG）不输出 trace 行——它们的逐文件结果行已经包含了所有有意义的信息。逐文件结果行和汇总不会被改变；trace 是纯粹叠加的。

### `--summary-only`

传 `--summary-only` 抑制 stdout 上的逐文件 `saved/skipped` 行。汇总仍然打印，失败仍输出到 stderr。当 CI 只关心总量时很有用：

```
$ imageoptim --summary-only assets/**/*.png
Processed 47 files, saved 12.34 MB (38.21%)
```

`tests/example01.png`（2.89 MB 合成 1122×1402 RGB 照片，由 `cargo run --example gen-fixtures` 生成；该文件被 .gitignore——见 [Development](#development) 下的 "Test fixtures"）上的实测数据：

| 路径 | 输出 | 节省 |
| --- | --- | --- |
| `--png-optimization-level 0`（最快） | 799 KB | 72.34% |
| 默认（`oxipng` 预设 3） | 1.97 MB | 14.86%（由预设 0 推断；合成图像以噪声为主） |
| `--lossy`（pngquant + oxipng max + oxipng 内置 zopfli） | 560 KB | 80.61% |
| `--lossy --max-colors 128` | 478 KB | 83.45% |
| `--lossy --max-colors 16` | 被拒绝 | imagequant 在这张充满噪声的合成图上无法用 16 色满足 80-100 质量目标；真实照片可压得更小（之前实测 286 KB / 87.67%） |
| `--lossy` 且安装 `zopflipng`（估算） | ~250 KB | ~91% |

### 备份（默认开启）

在覆盖之前，`imageoptim-rs` 把原文件复制为 `<path>.bak`。备份在每个文件的**首次**运行创建，且后续运行不会覆盖它。要从备份恢复：

```bash
mv foo.png.bak foo.png
```

`--dry-run` 模式跳过备份；用 `--no-backup` 可彻底关掉（文件仍然会就地优化，只是不再创建 `.bak`）。备份与原文件位于同一目录，因此首次优化跑完后文件数量大约会翻倍——确认优化效果后记得清理。

### 输出目录（非就地写入）

`--output-dir <DIR>` 把每个优化后的文件输出到 `<DIR>/<stem>_s<ext>`，而不是就地覆盖原文件。原文件保持不动，所以 `--no-backup` 是隐含的，也不会产生 `.bak` 文件。目标目录若不存在会自动创建。

若 `<stem>_s<ext>` 已存在于 `<DIR>`，会追加数字后缀：`foo_s-1.png`、`foo_s-2.png`……永远不会被静默覆盖。

```bash
# 横向对比：原文件与优化结果并排
imageoptim assets/*.png --output-dir out/

# assets/foo.png  →  out/foo_s.png
# assets/bar.jpg  →  out/bar_s.jpg
```

这是在决定就地覆盖之前做 A/B 对比的推荐用法。

### Fail-fast

默认情况下即使某些文件出错也会继续处理，最终退出码为 1（如果有任何文件失败）。传 `--fail-fast` 在第一次出错时立即短路并退出。在 CI 流水线里——任何失败都应中止构建——这一参数很有用。

## 开发

### 测试 fixture

集成测试读取一张位于 `tests/example01.png` 的 2-3 MB 照片 fixture。为了让仓库保持精简，该文件被 .gitignore，并在需要时由一个示例程序生成：

```sh
cargo run --example gen-fixtures
```

这会写入一张 1122×1402 RGB PNG，它的形态像自然照片（平滑的色彩区域加低幅度噪声），让有损调色板量化器有真正的工作可做。输出是确定性的——使用了带种子的 LCG——所以字节在不同运行、不同平台之间保持一致。

跳过这一步，依赖 fixture 的测试会静默跳过（打印 `skipping: tests/example01.png not present` 并返回 0），其他测试照常运行。要端到端跑完整套件：

```sh
cargo run --example gen-fixtures && cargo test
```

若 `tests/example01.png` 已存在，`gen-fixtures` 会拒绝覆盖（这是一次对 2.89 MB fixture 的破坏性操作——你可能是有意重新生成的）。传 `--force` 强制覆盖：

```sh
cargo run --example gen-fixtures -- --force
```

### 跨平台编译

代码是可移植的纯 Rust：没有 `#[cfg(target_os = ...)]` 误匹配，没有 `sh -c` shell-out，路径处理统一使用 `std::path::Path`。唯一的平台相关分支在 `which()` 的 Windows 扩展名探测（`#[cfg(windows)]` 用于 `.exe` / `.bat` / `.cmd`）。

GitHub Actions CI 在 `ubuntu-latest` / `macos-latest` / `windows-latest` 上对 `main` 的每次 push 与 PR 构建并测试（见 `.github/workflows/ci.yml`）。release workflow（`.github/workflows/release.yml`）在每次 `v*.*.*` tag push 时交叉编译 `x86_64-unknown-linux-gnu`、`x86_64-apple-darwin`、`aarch64-apple-darwin`、`x86_64-pc-windows-msvc` 的预编译二进制，并把制品挂到 GitHub Release。

本地交叉编译（无 CI）：

- 安装目标平台的 std 库：`rustup target add <triple>`。需要能访问 `static.rust-lang.org`。
- 然后 `cargo check --target <triple>` 验证本 crate 代码能编译到该目标。首次构建陌生目标会下载 sysroot（~100 MB），初次运行可能要 1-2 分钟。
- 真二进制还需要平台 C 工具链（Windows GNU 用 `mingw-w64`，Windows MSVC 用 MSVC build tools，Linux 静态二进制用 `musl-tools`）。

## 与 ImageOptim-CLI 的对比

| | ImageOptim-CLI | imageoptim-rs |
| --- | --- | --- |
| 平台 | 仅 macOS | macOS、Linux、Windows |
| 运行时依赖 | 三个 macOS GUI 应用 | 无（单一静态二进制） |
| 实现 | TypeScript + AppleScript | 纯 Rust |
| 维护 | 2023-11 起归档 | 活跃 |
| 格式自动识别 | 是 | 是 |
| Glob 支持 | 是 | 是 |
| 递归 | 是 | 是 |
| Dry-run | 否 | 是 |
| 跨格式转换（PNG→WebP） | 否 | 否（不在范围内） |

## 许可证

GPL-3.0-or-later — 见 `LICENSE`。链接本代码的二进制（含默认构建里启用 PNG 有损路径的版本）也必须以 GPL-3.0-or-later 分发，并且必须向接收者提供源代码。

## 致谢

- [`JamieMason/ImageOptim-CLI`](https://github.com/JamieMason/ImageOptim-CLI) —— 最初的概念与 CLI 形态
- 所有让这一切成为可能的 Rust crate 作者：`oxipng`、`gif`、`webp`、`usvg`、`image`、`jpeg-decoder`、`jpeg-encoder`
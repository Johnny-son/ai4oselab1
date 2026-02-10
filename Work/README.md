# Work

本仓库的产出会逐步沉淀到 `Work/` 目录下。

## 当前进度（2026-02-10）

- ✅ 已在 `Work/os` 下跑通前三章：ch1 / ch2 / ch3
- ✅ 采用组件化结构：可复用模块拆到 `Work/crates/*`
- ✅ 运行成功（每章自带 `tg-user`，用于构建用户程序）

## 目录结构

- `Work/os/`：内核/实验主体（Cargo workspace）
	- `ch1/`、`ch2/`、`ch3/`：章节内核
	- 每章包含 `tg-user/`：build.rs 构建用户程序用
- `Work/crates/`：拆出来的可复用组件（crate）
	- 目前已引入：`tg-sbi`、`tg-linker`、`tg-console`、`tg-kernel-context`、`tg-syscall`、`tg-signal-defs` 等

## 🚀 运行方式（ch1~ch3）

建议直接进入对应章目录运行（最不容易踩坑）：

### ch1

```bash
cd Work/os/ch1
cargo run
```

### ch2

```bash
cd Work/os/ch2
cargo run
```

### ch3

```bash
cd Work/os/ch3
cargo run
```

> 说明：`cargo run` 会使用 QEMU 启动 RISC-V 内核镜像。若你想了解 workspace 配置、QEMU runner、`TG_USER_DIR` 等细节，请看 `Work/os/README.md`。

## 常见问题（简单版）

- 为什么有时是 `cargo run -p tg-chX`，不是 `-p chX`？
	- `-p/--package` 用的是 `[package].name`，不是目录名。
	- 所以推荐你在章节目录里直接 `cargo run`。

- build.rs 报 “no bin target named 00hello_world” 怎么办？
	- 优先检查是否在章节目录运行（`Work/os/ch2` 或 `Work/os/ch3`）。
	- 每章的 `tg-user/` 是独立 workspace（manifest 里有空的 `[workspace]`），用于避免被父 workspace 影响。

## 你需要最终具备的内容（Checklist）

- 实验指导文档（Markdown，可配 mermaid 图）
- 实验代码（Rust，目标 RISC-V 64，可在 QEMU 上运行）
- 测试用例（单元测试/系统测试）
- 组件化：把可复用模块拆成 crate（必要时发布到 crates.io 或保持本地 workspace 依赖）

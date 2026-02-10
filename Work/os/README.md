# Work/os

这是你自己的 rCore 教学实验工作区（Cargo workspace）。这里的目标是：**ch1~ch3 可以直接在本目录下编译并用 QEMU 运行**，并且组件（`Work/crates/*`）可以被你随时 hack 修改。

## 目录约定

- `ch1/`、`ch2/`、`ch3/`：章节内核（目录名按你的要求不带 `tg-`）
- 每章目录下都有一个 `tg-user/`：用于 build.rs 构建用户态程序（会被打包进内核镜像）
- workspace 根：`Work/os/Cargo.toml`（统一 profile 与 workspace 依赖）

## 🚀 运行（已验证）

从任意章节目录运行都可以：

- 运行 ch1：

```bash
cd Work/os/ch1
cargo run
```

- 运行 ch2：

```bash
cd Work/os/ch2
cargo run
```

- 运行 ch3：

```bash
cd Work/os/ch3
cargo run
```

### 不再依赖 Reference

已把 `tg-user/` 放在每章目录内，并在 `Work/os/.cargo/config.toml` 里把 `TG_USER_DIR` 默认设置为 `./tg-user`（相对当前 crate）。所以：

- 不需要 `Reference/`
- 不需要手动 export `TG_USER_DIR=...`

如果你确实想临时切换到别的用户程序集合，再手动指定也行：

```bash
TG_USER_DIR=/abs/path/to/another/tg-user cargo run
```

## 为什么是 `cargo run -p tg-ch1`（不是 `-p ch1`）？

`-p/--package` 用的是 **Cargo.toml 里的 `[package].name`**，不是目录名。

当前这套移植里：目录叫 `ch1/`，但包名可能仍然是上游习惯（比如 `tg-ch1`）或你后来改成了 `ch1`；所以：

- 用 `cargo run`（在章节目录里）最省心
- 或者在 workspace 根用 `cargo run -p <package.name>`

如果你希望 workspace 根也能写成 `cargo run -p ch1`，就需要把每章 `Cargo.toml` 的 `[package].name` 调整为 `ch1/ch2/ch3`（这会影响依赖名、文档名等，属于“可做但会产生连锁改动”，建议等前三章完全稳定后再做）。

## 常见坑

- build.rs 报 “no bin target named 00hello_world”
	- 通常是 `TG_USER_DIR` 指错了，或对应章的 `tg-user/` 没被当成独立 workspace。
	- 目前我们已经在 `ch2/tg-user/Cargo.toml` 和 `ch3/tg-user/Cargo.toml` 里加了空的 `[workspace]`，用于隔离父 workspace。

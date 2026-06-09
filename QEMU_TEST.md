# QEMU 测试指南

## 🎯 快速开始

### 1. 安装依赖

```bash
# Arch Linux
sudo pacman -S qemu-full edk2-ovmf

# Ubuntu/Debian
sudo apt install qemu-system-x86 ovmf

# Fedora
sudo dnf install qemu-system-x86 edk2-ovmf
```

### 2. 构建项目

```bash
./scripts/build.sh
```

### 3. 运行 QEMU

```bash
./scripts/run-qemu.sh
```

## 🔧 高级选项

### 显示后端

```bash
# GTK 显示（默认，推荐）
QEMU_DISPLAY=gtk ./scripts/run-qemu.sh

# SDL 显示
QEMU_DISPLAY=sdl ./scripts/run-qemu.sh

# VNC（远程访问）
QEMU_DISPLAY=vnc=:0 ./scripts/run-qemu.sh

# 无头模式（仅串口输出）
QEMU_DISPLAY=none ./scripts/run-qemu.sh
```

### 启用 KVM（性能提升）

```bash
ENABLE_KVM=1 ./scripts/run-qemu.sh
```

### 启用 USB 键盘

```bash
ENABLE_USB=1 ./scripts/run-qemu.sh
```

### 自定义 OVMF 路径

```bash
OVMF_PATH=/path/to/OVMF.fd ./scripts/run-qemu.sh
```

## 📊 调试功能

### 串口日志

QEMU 会将串口输出保存到：
```
./qemu_serial.log
```

查看实时日志：
```bash
tail -f qemu_serial.log
```

### QEMU 监视器

在 QEMU 窗口中按 `Ctrl+Alt+2` 切换到监视器，可以：
- 查看寄存器状态
- 设置断点
- 转储内存

### GDB 调试

启动 QEMU 并等待 GDB 连接：
```bash
qemu-system-x86_64 \
    -bios /usr/share/edk2/x64/OVMF.4m.fd \
    -drive "format=raw,file=fat:rw:./esp" \
    -display gtk \
    -s -S
```

在另一个终端连接 GDB：
```bash
gdb
(gdb) target remote :1234
(gdb) continue
```

## 🎮 游戏控制

在 QEMU 窗口中：
- **方向键** - 选择骰子/菜单导航
- **Enter/Space** - 确认/掷骰子
- **B** - 存分
- **R** - 掷骰子
- **Q/Esc** - 退出

## 🐛 常见问题

### 1. OVMF 固件找不到

```
❌ OVMF firmware not found!
```

**解决方案**：
```bash
# 检查已安装的固件
find /usr -name "OVMF*.fd" 2>/dev/null

# 或者手动指定路径
OVMF_PATH=/usr/share/edk2-ovmf/x64/OVMF.4m.fd ./scripts/run-qemu.sh
```

### 2. 显示窗口不出现

**解决方案**：
```bash
# 尝试不同的显示后端
QEMU_DISPLAY=sdl ./scripts/run-qemu.sh

# 或者检查是否有 X11/Wayland
echo $DISPLAY
```

### 3. 键盘无响应

**解决方案**：
```bash
# 启用 USB 键盘
ENABLE_USB=1 ./scripts/run-qemu.sh

# 或者点击 QEMU 窗口获取焦点
```

### 4. 性能太差

**解决方案**：
```bash
# 启用 KVM（需要 root 权限）
sudo ENABLE_KVM=1 ./scripts/run-qemu.sh

# 或者减少 QEMU 资源占用
qemu-system-x86_64 -m 128M -smp 1 ...
```

### 5. 黑屏或无显示

**检查清单**：
- [ ] 构建是否成功？
- [ ] ESP 目录是否有 BOOTX64.EFI？
- [ ] OVMF 固件是否正确？
- [ ] 串口日志是否有输出？

```bash
# 检查构建输出
ls -la esp/EFI/BOOT/

# 查看串口日志
cat qemu_serial.log
```

## 📝 调试技巧

### 1. 添加启动延迟

在 `main.rs` 的 `main()` 函数开始处添加：
```rust
// 等待 2 秒，方便查看启动信息
uefi::boot::stall(core::time::Duration::from_secs(2));
```

### 2. 输出调试信息到串口

使用 `debug_log!` 宏，日志会写入：
- U 盘上的 `\frakle_debug.log` 文件
- 串口输出（如果配置了串口）

### 3. 截图/QEMU 快照

```bash
# 截图
Ctrl+Alt+2
(qemu) screendump screenshot.ppm

# 保存快照
(qemu) savevm my_snapshot

# 恢复快照
(qemu) loadvm my_snapshot
```

### 4. 内存转储

```bash
# 在 QEMU 监视器中
(qemu) pmemsave 0x1000 0x100000 memory_dump.bin
```

## 🔄 开发工作流

```bash
# 1. 修改代码
vim src/main.rs

# 2. 快速构建测试
cargo build --release --target x86_64-unknown-uefi

# 3. 在 QEMU 中测试
./scripts/run-qemu.sh

# 4. 查看日志
tail -f qemu_serial.log

# 5. 重复直到满意

# 6. 部署到真机
sudo ./scripts/deploy-usb.sh
```

## 📚 相关资源

- [QEMU 文档](https://www.qemu.org/docs/master/)
- [OVMF Wiki](https://github.com/tianocore/tianocore.github.io/wiki/OVMF)
- [UEFI 规范](https://uefi.org/specifications)

---

**提示**：QEMU 测试比真机测试更快、更安全，建议先在 QEMU 中调试所有功能！


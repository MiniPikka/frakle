# Farkle 调试指南

## 调试手段

### 1. 屏幕实时调试覆盖层

游戏左上角显示：
- **绿色**: `F:帧数 PhaseName` — 帧计数 + 当前游戏阶段
- **黄色**: 双方分数 + 当前回合分
- **红色 STUCK!**: 阶段卡死超过 10 秒（600 帧）

崩溃时拍下屏幕就能看到：崩溃帧数、阶段、是否有死循环。

### 2. QEMU 本地测试

```bash
# 桌面环境
bash scripts/run-qemu.sh

# 无头模式
env QEMU_DISPLAY=none bash scripts/run-qemu.sh

# GDB 远程调试
bash scripts/run-qemu-gdb.sh
# 另一个终端:
gdb -ex "target remote :1234" target/x86_64-unknown-uefi/debug/frakle.efi
```

QEMU 脚本使用 pflash + mtools 创建的 FAT16 镜像（非 `fat:rw:`），避免 QEMU vvfat 驱动崩溃。
`cache=unsafe` 使日志写入更快刷新到宿主文件。

### 3. QEMU CPU 异常日志

```bash
qemu-system-x86_64 ... -d int,cpu_reset -D crash.log
```

日志格式：`v=XX` 表示中断向量。常见值：
- `v=0e` — Page Fault (#PF)
- `v=0d` — General Protection Fault (#GP)
- `v=06` — Undefined Opcode (#UD)
- `v=20` — APIC Timer (正常)

### 4. 文件日志

Logger 在 ESP 分区写入 `\frakle_debug.log`。记录内容：
- 启动信息（Game struct 大小等）
- 每次阶段切换（Title → P:Roll? → P:Select → AI:Think → …）
- 每次按键事件
- 游戏结束（胜者、最终分数）
- 每 600 帧心跳（确认游戏循环存活）

**QEMU 读取日志**（需 `cache=unsafe`，QEMU 磁盘缓存定期刷新）：
```bash
mcopy -i esp.img ::frakle_debug.log /dev/stdout
```

**真机读取日志**：U 盘插入 Linux 后直接查看 `\frakle_debug.log`。

### 5. 查看 GDB 崩溃现场

```bash
gdb -batch \
    -ex "target remote :1234" \
    -ex "info registers" \
    -ex "x/30i \$rip-20" \
    target/x86_64-unknown-uefi/debug/frakle.efi
```

## GDB 调试结果

在 QEMU 中捕获到的 "暂停" 状态：
- RIP = `0xf6e23f9`（游戏代码内，非固件）
- GDB 显示 `add $0x38,%rsp` — 正常栈帧清理指令
- 寄存器状态正常（RSP 有效，无野指针）
- QEMU 异常日志中**无任何 CPU 异常**（全是正常定时器中断 v=20）

结论：游戏在 QEMU (OVMF) 中不会触发 CPU 异常。实机崩溃可能是固件 GOP 驱动兼容性问题。

## 已知问题

### 实机黑屏崩溃

**症状**: 玩一段时间后黑屏，重启回到 BIOS/UEFI 设置

**QEMU 测试**: 无法复现（OVMF 固件中正常运行 30+ 秒）

**可能原因**: 实机 UEFI 固件的 GOP 驱动对直接帧缓冲写入不兼容

**已尝试的修复**:
1. ~~堆内存碎片~~ → 热路径零分配
2. ~~`gop.blt()` 固件 bug~~ → 改为直接写 GOP 帧缓冲（`copy_nonoverlapping` 按行复制）
3. ~~RNG 固定种子~~ → RDTSC 随机种子
4. 添加了 panic handler、阶段看门狗、帧计数器屏幕叠加层

## 项目质量

| 指标 | 状态 |
|------|------|
| Clippy (主 crate) | 零 warning |
| unsafe 代码 | 3 行（`ptr::copy_nonoverlapping` + `_rdtsc`） |
| 热路径堆分配 | 0 |
| 游戏逻辑测试 | 15/15 通过 |
| QEMU 运行 | 稳定 |
| 二进制大小 | ~80KB (release) |

# Farkle UEFI Game - 部署到 U 盘指南

## 快速开始

### 1. 构建项目

```bash
cd /home/zxl/Documents/myprojects/frakle
./scripts/build.sh
```

构建完成后，EFI 文件会生成在：
- `esp/EFI/BOOT/BOOTX64.EFI` (自动启动)
- `esp/EFI/Farkle/Farkle.efi` (备份)

### 2. 部署到 U 盘

**⚠️ 警告：此操作会清除 U 盘上的所有数据！**

#### 方法一：使用部署脚本（推荐）

```bash
# 交互式选择设备
sudo ./scripts/deploy-usb.sh

# 或者直接指定设备（你的 U 盘是 /dev/sda）
sudo USB_DEV=/dev/sda ./scripts/deploy-usb.sh
```

脚本会自动：
- 构建项目
- 创建 GPT 分区表
- 格式化为 FAT32
- 复制 EFI 文件
- 设置可启动标志

#### 方法二：手动部署

如果脚本有问题，可以手动操作：

```bash
# 1. 确认 U 盘设备
lsblk -o NAME,SIZE,MODEL,TRAN | grep usb

# 2. 卸载已挂载的分区
sudo umount /dev/sda* 2>/dev/null || true

# 3. 创建分区（会清除所有数据！）
sudo sgdisk --zap-all /dev/sda
sudo sgdisk --new=1:0:0 --typecode=1:ef00 /dev/sda

# 4. 格式化为 FAT32
sudo mkfs.fat -F 32 -n "FARKLE" /dev/sda1

# 5. 挂载并复制文件
sudo mkdir -p /mnt/usb
sudo mount /dev/sda1 /mnt/usb
sudo mkdir -p /mnt/usb/EFI/BOOT
sudo cp esp/EFI/BOOT/BOOTX64.EFI /mnt/usb/EFI/BOOT/
sudo umount /mnt/usb
```

### 3. 在真机上启动

1. **安全弹出 U 盘**
   ```bash
   eject /dev/sda
   ```

2. **插入目标电脑**

3. **进入 BIOS/UEFI 设置**
   - 开机时按 F2、F12、DEL 或 ESC（取决于主板型号）
   - 找到 Boot Order / Boot Priority 设置

4. **配置启动顺序**
   - 将 USB 设备设置为第一启动项
   - 保存并退出

5. **禁用 Secure Boot**（如果需要）
   - 在 BIOS 的 Security 选项中禁用 Secure Boot
   - 保存并退出

6. **启动游戏**
   - 电脑会从 U 盘启动
   - Farkle 游戏会自动运行

## 游戏玩法

Farkle 是一个骰子游戏：

1. **开始回合** - 掷 6 个骰子
2. **选择得分骰子** - 点击要保留的骰子（1、5 或三个相同）
3. **决定继续或停止**
   - 选择 "Bank" 保存当前回合得分
   - 选择 "Roll" 用剩余骰子继续掷
4. **Farkle!** - 如果没有得分组合，回合结束，失去本轮得分
5. **目标** - 第一个达到 10,000 分获胜

### 得分规则

- **1** = 100 分
- **5** = 50 分
- **三个相同** = 数值 × 100（例如三个 6 = 600）
- **三个 1** = 1000 分

## 故障排除

### U 盘无法启动

1. 检查 BIOS 设置：
   - 确认 U 盘是第一启动项
   - 确认禁用了 Secure Boot
   - 确认启用了 Legacy Boot 或 UEFI Boot

2. 检查 U 盘格式：
   ```bash
   sudo fdisk -l /dev/sda
   ```
   应该显示 GPT 分区表和 EFI System 分区

3. 重新制作 U 盘：
   ```bash
   sudo USB_DEV=/dev/sda ./scripts/deploy-usb.sh
   ```

### 游戏无法显示

1. 确认显示器连接正常
2. 尝试不同的显示输出（HDMI、DP、VGA）
3. 检查是否需要特殊显卡驱动（UEFI 标准 VGA 通常兼容）

### 键盘/鼠标无响应

1. 尝试不同的 USB 端口
2. 使用有线键盘/鼠标（无线可能需要额外驱动）
3. 检查 BIOS 中的 USB Legacy Support 设置

## 技术细节

- **目标架构**: x86_64 UEFI
- **Rust 工具链**: nightly
- **UEFI 库**: uefi 0.37
- **图形库**: embedded-graphics 0.8
- **分辨率**: 自动检测（通常 800x600 或更高）

## 重新构建和部署

如果修改了代码：

```bash
# 重新构建
cargo build --release --target x86_64-unknown-uefi

# 重新部署到 U 盘
sudo USB_DEV=/dev/sda ./scripts/deploy-usb.sh
```

## 从 U 盘移除游戏

如果你想清除 U 盘上的游戏并恢复正常使用：

```bash
# 重新分区（会清除所有数据！）
sudo sgdisk --zap-all /dev/sda
sudo mkfs.fat -F 32 -n "MYUSB" /dev/sda1
```

## 获取帮助

- 查看项目 README: `cat README.md`
- 查看实现计划: `cat implementation-plan.md`
- 查看技术栈: `cat tech-stack.md`

---

**祝你玩得开心！🎲**
# 开发命令指南

## 🚀 快速开始

### 开发模式（推荐）
```bash
npm run tauri:dev
```
启动完整的 Tauri 开发环境，包括：
- Vite 前端热重载
- Rust 后端自动编译
- 应用窗口实时更新

### 仅前端开发
```bash
npm run dev
```
仅启动 Vite 开发服务器，用于 UI 开发

---

## 📦 构建命令

### 生产构建
```bash
npm run tauri:build
```
构建优化的生产版本：
- macOS: `src-tauri/target/release/bundle/dmg/`
- Windows: `src-tauri/target/release/bundle/msi/`
- Linux: `src-tauri/target/release/bundle/appimage/`

### Debug 构建
```bash
npm run tauri:build:debug
```
构建 debug 版本（保留调试信息）

---

## 🔍 检查命令

### TypeScript 类型检查
```bash
npm run check
```
检查 TypeScript 类型错误（不生成文件）

### Rust 代码检查
```bash
npm run check:rust
```
检查 Rust 代码编译错误

---

## 🎨 代码质量

### 格式化代码
```bash
npm run format
```
使用 Prettier 格式化所有前端代码

---

## 🧹 清理命令

### 清理所有构建文件
```bash
npm run clean
```
删除：
- `dist/` - 前端构建产物
- `src-tauri/target/` - Rust 构建产物
- `node_modules/` - npm 依赖

重新安装：
```bash
npm install
```

---

## 💡 常用工作流

### 开发新功能
```bash
# 1. 启动开发服务器
npm run tauri:dev

# 2. 修改代码（自动热重载）

# 3. 检查类型
npm run check

# 4. 格式化代码
npm run format
```

### 发布前检查
```bash
# 1. 类型检查
npm run check

# 2. Rust 检查
npm run check:rust

# 3. 构建测试
npm run tauri:build

# 4. 测试构建产物
```

---

## 🐛 调试技巧

### 查看控制台日志
开发模式下，打开浏览器开发者工具：
- macOS: `Cmd + Option + I`
- Windows/Linux: `Ctrl + Shift + I`

### 查看 Rust 日志
开发模式下，终端会显示 Rust 日志：
```bash
RUST_LOG=debug npm run tauri:dev
```

---

## 📚 更多信息

- Tauri 文档: https://tauri.app
- Vite 文档: https://vite.dev
- React 文档: https://react.dev

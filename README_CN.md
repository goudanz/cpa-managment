# CPAPersonal

![CPAPersonal Logo](./assets/logomain.png)

一个面向 CLIProxyAPI 工作流的本地优先桌面管理器。

## 功能特性

- 提供服务启停、Provider、认证文件、密钥策略的一体化桌面管理
- 内置日志页、用量统计与 GitHub Release 轻量更新能力
- 中英双语界面，含开机自启动与日志/统计开关

## 仓库地址

- 项目仓库：https://github.com/WEP-56/CPAPersonal
- 上游项目：https://github.com/router-for-me/CLIProxyAPI

## 桌面端打包

```bash
cd desktop
npm install
npm run tauri build
```

Windows 打包图标来源：`assets/logo1.ico`（已同步到 `desktop/src-tauri/icons/icon.ico`）。

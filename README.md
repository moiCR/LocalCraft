# 🏗️ LocalCraft

<div align="center">
  <img src="icon.png" width="128" height="128" alt="LocalCraft Logo">
  <p align="center">
    <strong>A high-performance, modern Minecraft server manager built with Tauri and Vue.</strong>
  </p>

  [![Version](https://img.shields.io/badge/version-0.1.0-blue.svg)](https://github.com/your-username/localcraft-vuew)
  [![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
  [![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20Windows%20%7C%20macOS-lightgrey.svg)](#)
</div>

## ✨ Introduction

LocalCraft is a desktop application designed to simplify the process of hosting and managing Minecraft servers locally. Leveraging the power of **Tauri** for a lightweight backend and **Vue 3** for a sleek, responsive UI, it provides a premium experience for both casual players and power users.

## 🚀 Current Features

- **Java Management**: Automatic detection and download of different JRE versions.
- **Server Creation**: Intuitive wizard to configure software (Vanilla, Paper, Fabric, etc.) and versions.
- **Modern Interface**: Fluid animations with GSAP and minimalist design.
- **Cross-platform**: Compatible with Linux, Windows, and macOS thanks to Tauri.

## 🗺️ Roadmap

This is our roadmap to reach version 1.0.

### ✅ Completed
- [x] Base structure with Tauri and Vue 3.
- [x] Java management system (Java Manager).
- [x] Server creation interface (`ServerCreateModal`).
- [x] Main server display dashboard.

### 🚧 In Development / Coming Soon
- [x] **Server Lifecycle Management**:
  - [x] Start, Stop, and Restart buttons.
  - [x] Process state (Running/Stopped).
- [x] **Real-time Console**:
  - [x] Log stream directly from the server process.
  - [x] Command input to the console.
- [ ] **File Manager**:
  - [ ] Integrated file explorer.
  - [ ] Text editor for `.properties`, `.yml`, `.json` files.
- [ ] **Mods/Plugins Section**:
  - [ ] Integration with the **Modrinth** API.
  - [ ] Integration with the **CurseForge** API.
  - [ ] One-click installation.

---

## 📸 Image Gallery

Below are the key components of the application:

| Component | Screenshot |
| :--- | :--- |
| **Dashboard** | ![Dashboard](public/servers_dashboard.png) |
| **Server Creation** | ![Creation Modal](public/servers_dashboard_with_modal.png) |
| **Java Configuration** | (Coming Soon) |

---

## 🛠️ Tech Stack

- **Frontend**: [Vue 3](https://vuejs.org/) + [Vite](https://vitejs.dev/) + [TypeScript](https://www.typescriptlang.org/)
- **Styling**: Vanilla CSS / Scoped CSS
- **Backend/Native**: [Tauri](https://tauri.app/) (Rust)
- **Animations**: [GSAP](https://greensock.com/gsap/)
- **Icons**: Internal SVGs + Lucide-Vue

## 📦 Installation (Development)

To run the project locally:

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/moiCR/LocalCraft.git
    ```
2.  **Install dependencies:**
    ```bash
    bun install
    ```
3.  **Run in dev mode:**
    ```bash
    bun tauri dev
    ```

---

<div align="center">
  Made with ❤️ by Moisés Marenco Vives
</div>

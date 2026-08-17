#!/bin/bash

# AI 提示本地环境的 LinuxDeploy 工具过老……
export NO_STRIP=1

bun run tauri build

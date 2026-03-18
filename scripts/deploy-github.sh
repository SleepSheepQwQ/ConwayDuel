#!/bin/bash
set -e

# 本地只提交代码，不编译，云端 GitHub Actions 自动处理
echo "=== 推送代码到 GitHub，云端自动编译部署 ==="
git add .
git commit -m "Deploy geometry spaceship battle to GitHub Pages"
git push origin main

echo -e "\033[32m[✔] 代码已推送，等待 1~2 分钟，GitHub Pages 自动更新完成！\033[0m"

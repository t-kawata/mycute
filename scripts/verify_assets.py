import os
import subprocess
from pathlib import Path

# 設定
WORKSPACE_ROOT = "/Users/shyme01/shyme/mycute"
PUBLIC_DIR = os.path.join(WORKSPACE_ROOT, "web/public")
TARGET_DIR = os.path.join(WORKSPACE_ROOT, "web/tmp-assets")

def run_grep(pattern):
    # node_modules, target, dist, .git, tmp-assets, artifacts, scripts, log を除外
    cmd = [
        "grep", "-r", "--exclude-dir", "node_modules", 
        "--exclude-dir", "target", 
        "--exclude-dir", "dist", 
        "--exclude-dir", ".git", 
        "--exclude-dir", "tmp-assets", 
        "--exclude-dir", "artifacts",
        "--exclude", "analyze_assets.py",
        "--exclude", "verify_assets.py",
        "--exclude", "*.log",
        pattern, WORKSPACE_ROOT
    ]
    try:
        result = subprocess.run(cmd, capture_output=True, text=True)
        # ヒットした行数を数える（空行は除く）
        hits = [line for line in result.stdout.split('\n') if line.strip()]
        return len(hits)
    except Exception:
        return 0

def get_all_files(directory):
    file_list = []
    if not os.path.exists(directory):
        return file_list
    for root, _, files in os.walk(directory):
        for file in files:
            full_path = os.path.join(root, file)
            rel_path = os.path.relpath(full_path, directory)
            file_list.append(rel_path)
    return file_list

def main():
    print("--- 点検開始 ---")
    
    # 1. 残っているファイルの点検
    remaining_files = get_all_files(PUBLIC_DIR)
    suspicious_remaining = []
    
    print(f"Checking {len(remaining_files)} remaining files in web/public...")
    for rel_path in remaining_files:
        basename = os.path.basename(rel_path)
        
        # icons/ 配下は Quasar 管理のためスキップ（あるいは個別チェック）
        if rel_path.startswith("icons"):
            continue
            
        # 完全パスとベース名の両方で検索
        hit_count = run_grep(rel_path)
        if hit_count == 0:
            hit_count = run_grep(basename)
            
        if hit_count == 0:
            suspicious_remaining.append(rel_path)
            
    # 2. 退避されたファイルの点検
    evacuated_files = get_all_files(TARGET_DIR)
    erroneous_evacuated = []
    
    print(f"Checking {len(evacuated_files)} evacuated files in web/tmp-assets...")
    for rel_path in evacuated_files:
        basename = os.path.basename(rel_path)
        
        # 完全パスで検索（ベース名だと数字のみなどがヒットしすぎるため）
        hit_count = run_grep(rel_path)
        if hit_count > 0:
            erroneous_evacuated.append(rel_path)

    # 結果報告用
    print("\n--- 点検結果報告 ---")
    print(f"[点検1] web/public に残っているが、参照が見つからなかったファイル (計 {len(suspicious_remaining)} 件):")
    for f in suspicious_remaining:
        print(f"  - {f}")
        
    print(f"\n[点検2] web/tmp-assets に退避されたが、参照が見つかったファイル (計 {len(erroneous_evacuated)} 件):")
    for f in erroneous_evacuated:
        print(f"  - {f}")

if __name__ == "__main__":
    main()

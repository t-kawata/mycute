import os
import shutil
from pathlib import Path

# 設定
WORKSPACE_ROOT = "/Users/shyme01/shyme/mycute"
PUBLIC_DIR = os.path.join(WORKSPACE_ROOT, "web/public")
TARGET_DIR = os.path.join(WORKSPACE_ROOT, "web/tmp-assets")

# スキャン対象とする拡張子
SCAN_EXTENSIONS = (
    '.vue', '.ts', '.js', '.rs', '.json', '.md', '.toml', '.yaml', '.yml', 
    '.scss', '.css', '.html', '.plist', '.sh', 'Makefile'
)

# 動的参照が確定しているディレクトリ（これらは中身を保持）
DYNAMIC_ASSET_DIRS = ["sample-face", "sample-img", "icons"]

def get_all_public_files(directory):
    file_list = []
    if not os.path.exists(directory):
        return file_list
    for root, _, files in os.walk(directory):
        for file in files:
            full_path = os.path.join(root, file)
            rel_path = os.path.relpath(full_path, directory)
            file_list.append(rel_path)
    return file_list

def get_total_search_content():
    content = ""
    exclude_dirs = {'node_modules', 'target', 'dist', '.git', 'web/dist', 'tmp-assets'}
    
    print("Scanning entire workspace for precise references...")
    for root, dirs, files in os.walk(WORKSPACE_ROOT):
        # 除外ディレクトリ
        dirs[:] = [d for d in dirs if os.path.relpath(os.path.join(root, d), WORKSPACE_ROOT) not in exclude_dirs]
        
        for file in files:
            if file.endswith(SCAN_EXTENSIONS) or file == 'Makefile':
                full_path = os.path.join(root, file)
                try:
                    with open(full_path, 'r', encoding='utf-8', errors='ignore') as f:
                        content += f.read() + "\n"
                except Exception as e:
                    pass
    return content

def main():
    if not os.path.exists(TARGET_DIR):
        os.makedirs(TARGET_DIR)

    public_files = get_all_public_files(PUBLIC_DIR)
    search_content = get_total_search_content()
    
    used_files = []
    unused_files = []

    for rel_path in public_files:
        basename = os.path.basename(rel_path)
        is_used = False
        
        # 1. 動的参照ディレクトリに含まれるか
        for d in DYNAMIC_ASSET_DIRS:
            if rel_path.startswith(d + os.sep) or rel_path == d:
                is_used = True
                break
        
        if not is_used:
            # 2. コンテンツ内での「完全一致」に近い参照確認
            # ファイルの相対パスが含まれているか（例: "icons/favicon-128x128.png"）
            if rel_path in search_content:
                is_used = True
            # ファイル名そのものが含まれているか
            elif basename in search_content:
                # ただし、短すぎる名前（数字のみなど）は誤検知の可能性があるため、
                # アセットらしい拡張子を持つ場合のみ判定（安全策）
                if len(basename) > 4 and basename in search_content:
                    is_used = True
        
        if is_used:
            used_files.append(rel_path)
        else:
            unused_files.append(rel_path)

    print(f"Total files: {len(public_files)}")
    print(f"Used (Protected): {len(used_files)}")
    print(f"Unused (To be moved): {len(unused_files)}")

    # 全て一度復旧されている前提で移動を実行
    for rel_path in unused_files:
        src_path = os.path.join(PUBLIC_DIR, rel_path)
        dst_path = os.path.join(TARGET_DIR, rel_path)
        
        if os.path.exists(src_path):
            os.makedirs(os.path.dirname(dst_path), exist_ok=True)
            shutil.move(src_path, dst_path)

    # ログ出力
    log_path = os.path.join(WORKSPACE_ROOT, "artifacts/evacuated_files_aggressive.log")
    os.makedirs(os.path.dirname(log_path), exist_ok=True)
    with open(log_path, 'w', encoding='utf-8') as f:
        f.write("\n".join(unused_files))
    print(f"Aggressive log saved to: {log_path}")

if __name__ == "__main__":
    main()

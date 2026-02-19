import json
import re
import os

def compact_specific_arrays(filepath):
    if not os.path.exists(filepath):
        print(f"File not found: {filepath}")
        return

    print(f"Processing {filepath}...")
    
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    # hotkeys の配列 [ "...", "..." ] を1行にする
    # replaces の各配列 [ "...", "..." ] を1行にする
    
    # 簡略化のため、すべての短い配列（ネストしたオブジェクトを含まない配列）を1行にする戦略
    def shrink_array(match):
        arr_str = match.group(0)
        # 配列の中にオブジェクト { } が含まれていない場合のみ1行にする
        if '{' not in arr_str:
            # 改行と余分なスペースを削除して1行にする
            items = json.loads(arr_str)
            return json.dumps(items, ensure_ascii=False)
        return arr_str

    # 配列 [...] を見つける正規表現 (最短一致)
    # 再帰的な構造は考慮せず、値としての配列を対象にする
    pattern = re.compile(r'\[[^\[\]\{]*?\]', re.DOTALL)
    
    new_content = pattern.sub(shrink_array, content)
    
    # 整形後、再度 JSON としてパースして全体をインデント2で書き直すと
    # 結局縦に戻ってしまう可能性があるため、文字列置換後の状態を維持して保存する
    
    # バックアップ
    os.replace(filepath, filepath + ".bak_format")
    
    with open(filepath, 'w', encoding='utf-8') as f:
        f.write(new_content)
    
    print(f"  Formatting completed. Original saved to {filepath}.bak_format")

if __name__ == "__main__":
    files = ["settings_mac.json", "settings_win.json"]
    for f in files:
        compact_specific_arrays(f)

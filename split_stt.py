import sys

with open('src/mode/cl/main_of_cl.rs', 'r') as f:
    lines = f.read().splitlines()

# Find the STT loop boundaries
start_stt_idx = -1
for i, line in enumerate(lines):
    if line.strip() == "let handle = app_handle_async.clone();" and lines[i+1].strip() == "let manager_for_stt = manager_async.clone();":
        start_stt_idx = i
        break

end_stt_idx = -1
for i in range(start_stt_idx, len(lines)):
    if line.strip() == "}); // STT Loop end":
        end_stt_idx = i
        break
    # Or find by specific text
    if "}); // STT Loop end" in lines[i]:
        end_stt_idx = i
        break

if start_stt_idx == -1 or end_stt_idx == -1:
    print(f"Error finding STT loop: start={start_stt_idx}, end={end_stt_idx}")
    sys.exit(1)

stt_loop_lines = lines[start_stt_idx:end_stt_idx+1]

# Find the injection point
inject_idx = -1
for i, line in enumerate(lines):
    if "デッドロック回避: 2つ目以降のウィンドウ生成とSTTループを非同期タスクに逃がす" in line:
        inject_idx = i
        break

if inject_idx == -1:
    print("Error finding injection point")
    sys.exit(1)

# Remove the STT loop from its original position
del lines[start_stt_idx:end_stt_idx+1]

# Prepare the separated STT loop
new_stt_loop = [
    "            // デッドロック回避 (Windows): WebView2ウィンドウ生成時、メッセージループの無いスレッドでブロックする問題がある。",
    "            // 運命共同体（Fate-sharing）を防ぎ、UIハング時でも音声入力が機能するようSTTイベントループを独立した非同期タスクとして分離・先行起動する。",
    "            let app_handle_for_stt = app.handle().clone();",
    "            let manager_for_stt = manager.clone();",
    "",
    "            // バックグラウンドタスク 1: STT イベントブリッジ (独立並行稼働)",
    "            async_runtime::spawn(async move {"
]

# We need to adapt the STT loop: change `handle` to `app_handle_for_stt`
for line in stt_loop_lines:
    if line.strip() == "let handle = app_handle_async.clone();":
        continue
    if line.strip() == "let manager_for_stt = manager_async.clone();":
        continue
    if line.strip() == "// バックグラウンドタスク: STT イベントブリッジ":
        continue
    if line.strip() == "async_runtime::spawn(async move {":
        continue
    
    # Replace handle.emit with app_handle_for_stt.emit
    modified_line = line.replace("handle.emit", "app_handle_for_stt.emit")
    
    # Adjust indentation: since we unnested it by one spawn? Actually no, it's still inside one spawn block. 
    # The original was inside a spawn block, so it was nested 2 times. Now it's nested once.
    # Original indentation of the inner content: 20 spaces (5 tabs). Let's remove 4 spaces.
    if modified_line.startswith("    "):
        modified_line = modified_line[4:]
    
    new_stt_loop.append(modified_line)

# Add spacing
new_stt_loop.append("")
new_stt_loop.append("            // バックグラウンドタスク 2: 追加のウィンドウ（オーバーレイ、スナックバー）生成")
new_stt_loop.append("            // ここでブロックが発生しても、タスク1（STT）は影響を受けない")

# Update the old injection point comments
lines[inject_idx] = "            let app_handle_async = app.handle().clone();"
lines[inject_idx+1] = "            let config_mgr_async = config_mgr.clone();"
if "let manager_async = manager.clone();" in lines[inject_idx+2]:
    del lines[inject_idx+2] # manager is no longer needed for UI creation

# Inject the new STT loop before the UI creation block (which starts at inject_idx)
lines.insert(inject_idx, "\n".join(new_stt_loop))

with open('src/mode/cl/main_of_cl.rs', 'w') as f:
    f.write("\n".join(lines) + "\n")

print("Successfully separated the STT loop!")

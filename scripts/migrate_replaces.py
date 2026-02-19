import json
import os
import sys

def migrate_file(filepath):
    if not os.path.exists(filepath):
        print(f"File not found: {filepath}")
        return

    print(f"Migrating {filepath}...")
    
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            data = json.load(f)
        
        if 'replaces' not in data:
            print(f"  No 'replaces' key found in {filepath}. Skipping.")
            return

        replaces = data['replaces']
        if not isinstance(replaces, dict):
            print(f"  'replaces' is not a dictionary. Skipping.")
            return

        # Check format
        is_old_format = False
        for k, v in replaces.items():
            if isinstance(v, str):
                is_old_format = True
                break
        
        if not is_old_format:
            print(f"  Already in new format (values are lists?) or empty. Skipping.")
            return

        print(f"  Old format detected. Converting...")
        
        new_replaces = {}
        for before, after in replaces.items():
            if isinstance(after, str):
                if after not in new_replaces:
                    new_replaces[after] = []
                new_replaces[after].append(before)
            else:
                # Handle mixed case if any? Or errors
                pass
        
        # Sort "befores" inside values? Maybe not needed but cleaner.
        for k in new_replaces:
            new_replaces[k].sort(key=len, reverse=True) # Longest first seems appropriate for matching logic, though here it's just storage.

        data['replaces'] = new_replaces
        
        # Create backup
        backup_path = filepath + ".bak_migration"
        with open(backup_path, 'w', encoding='utf-8') as f:
            json.dump(json.load(open(filepath, 'r', encoding='utf-8')), f, ensure_ascii=False, indent=2)
        print(f"  Backup saved to {backup_path}")

        # Save new file
        with open(filepath, 'w', encoding='utf-8') as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
        
        print(f"  Migration successful.")

    except Exception as e:
        print(f"  Error migrating {filepath}: {e}")

if __name__ == "__main__":
    target_files = [
        "settings_mac.json",
        "settings_win.json",
        # Also check default location?
        os.path.expanduser("~/.mycute/settings.json")
    ]
    
    # Check current directory files
    for fname in target_files:
        if os.path.exists(fname):
            migrate_file(fname)
        else:
            # try relative to script location or cwd
            pass

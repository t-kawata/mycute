#!/bin/zsh
# macos-setup.command
# This script removes the macOS "Quarantine" flag that causes the "damaged" error 
# for apps downloaded outside the App Store (ad-hoc signed apps).

# Determine the directory where the script is located
DIR=$(cd $(dirname $0); pwd)
APP_PATH="/Applications/NECO-ASOVI.app"

echo "------------------------------------------------------------"
echo " NECO-ASOVI: macOS App Setup Utility "
echo "------------------------------------------------------------"

if [ ! -d "$APP_PATH" ]; then
    echo "ERROR: NECO-ASOVI.app not found in /Applications folder."
    echo "Please copy the app to your Applications folder BEFORE running this script."
    echo ""
    echo "Press any key to exit..."
    read -k1 -s
    exit 1
fi

echo "Removing macOS Quarantine flag from $APP_PATH..."
echo "You may be prompted for your administrator password."
echo ""

# Use sudo to ensure we have permission to modify files in /Applications
sudo xattr -rd com.apple.quarantine "$APP_PATH"

if [ $? -eq 0 ]; then
    echo ""
    echo "------------------------------------------------------------"
    echo "SUCCESS: The Quarantine flag has been removed."
    echo "Attempting to launch NECO-ASOVI..."
    echo "------------------------------------------------------------"
    open -a "$APP_PATH"
else
    echo ""
    echo "ERROR: Failed to remove the flag. Please ensure you have admin rights."
fi

echo ""
echo "Press any key to close this window..."
read -k1 -s

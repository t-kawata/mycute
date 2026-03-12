import sys
import argparse
import os

def main():
    parser = argparse.ArgumentParser(description='Write content to a file.')
    parser.add_argument('--path', required=True, help='Path to the file to write')
    parser.add_argument('--content', required=True, help='Content to write to the file')
    args = parser.parse_args()

    os.makedirs(os.path.dirname(args.path), exist_ok=True)
    with open(args.path, 'w', encoding='utf-8') as f:
        f.write(args.content)

if __name__ == '__main__':
    main()

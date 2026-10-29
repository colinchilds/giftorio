import pyperclip
from pathlib import Path

Path("clipboard_content.txt").write_text(pyperclip.paste())

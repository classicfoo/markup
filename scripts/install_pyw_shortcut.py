import argparse
import os
import sys

import pythoncom
from win32com.propsys import propsys
from win32com.shell import shell


def absolute_path(path):
    return os.path.abspath(os.path.expanduser(path))


def start_menu_programs_dir():
    return os.path.join(
        os.environ["APPDATA"],
        "Microsoft",
        "Windows",
        "Start Menu",
        "Programs",
    )


def create_shortcut(app_script, app_id, name, icon):
    app_script = absolute_path(app_script)
    icon = absolute_path(icon)
    shortcut_path = os.path.join(start_menu_programs_dir(), f"{name}.lnk")

    if not os.path.isfile(app_script):
        raise FileNotFoundError(f"App script not found: {app_script}")
    if not os.path.isfile(icon):
        raise FileNotFoundError(f"Icon not found: {icon}")

    link = pythoncom.CoCreateInstance(
        shell.CLSID_ShellLink,
        None,
        pythoncom.CLSCTX_INPROC_SERVER,
        shell.IID_IShellLink,
    )
    link.SetPath(app_script)
    link.SetArguments("")
    link.SetWorkingDirectory(os.path.dirname(app_script))
    link.SetIconLocation(icon, 0)
    link.SetDescription(name)

    property_store = link.QueryInterface(propsys.IID_IPropertyStore)
    app_id_key = propsys.PSGetPropertyKeyFromName("System.AppUserModel.ID")
    property_store.SetValue(app_id_key, propsys.PROPVARIANTType(app_id))
    property_store.Commit()

    persist_file = link.QueryInterface(pythoncom.IID_IPersistFile)
    persist_file.Save(shortcut_path, 0)
    return shortcut_path


def main():
    parser = argparse.ArgumentParser(
        description="Install a Start Menu shortcut for an uncompiled .pyw app."
    )
    parser.add_argument("--app-script", default="markup.pyw")
    parser.add_argument("--app-id", default="michaelhuynh.screenshotmarkup.markup")
    parser.add_argument("--name", default="Screenshot Markup")
    parser.add_argument("--icon", default=os.path.join("icons", "markup.ico"))
    args = parser.parse_args()

    if sys.platform != "win32":
        raise SystemExit("This installer only creates Windows Start Menu shortcuts.")

    shortcut_path = create_shortcut(args.app_script, args.app_id, args.name, args.icon)
    print(shortcut_path)


if __name__ == "__main__":
    main()

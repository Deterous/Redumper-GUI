# Redumper GUI

A cross-platform digital fidget spinner and GUI for [redumper](https://github.com/superg/redumper).

<img width="1506" height="894" alt="tutorial" src="https://github.com/user-attachments/assets/59947cca-3d63-4b79-9282-861a2061d7a4" />

## Installation

Download the latest version for your OS from the [Releases](../../releases/latest) page.

The download contains both `redumper-gui` and `redumper` executables. Changing the bundled version of redumper is not recommended as it may not be supported by the GUI.

### Windows

Unzip the download to your location of choice and then double-click `redumper-gui.exe` to run.

If the Windows SmartScreen warning appears, click on **More info**, then **Run anyway**.

### Linux

Extract the `.tar.gz` archive to your location of choice and run the `redumper-gui` executable.

```sh
mkdir -p ~/Redumper-GUI
tar -xzf Redumper-GUI-Linux-x64.tar.gz -C ~/Redumper-GUI
~/Redumper-GUI/redumper-gui
```

### macOS

Open the dmg file in Finder, and move `Redumper GUI.app` to the Applications folder. After attempting to open the .app, macOS will warn you it could not verify the app as it is self-signed, you will have to go to "Privacy & Security" settings where it will say "Redumper GUI" was blocked to protect your Mac, then click 'Open Anyway'.

Alternatively you can first clear the protection setting in terminal with:

```sh
xattr -cr "Redumper GUI.app"
```

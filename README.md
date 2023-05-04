# `hui` - A visual alternative to `history`

<img src="./assets/hui_demo.gif" alt="Demo of hui">

## Overview


`hui` is command-line tool to quickly search through your terminal history. The motivation behind this tool was having a prettier and faster way to do `history | grep <search>`. I would do this frequently to remember some `docker` or `curl`
command I had done recently, but couldn't remember the flags I used. This now lets me search through my history and copy the command I want a lot easier.

`hui` is built on top of [ratatui](https://github.com/tui-rs-revival/ratatui) for its TUI, or Terminal User Interface.

## Setup

### Installation

If you are on a Mac, you can install from `homebrew`:

```bash
brew install hui
```

If you're on Ubuntu, you can install using `apt`:

```bash
apt install hui
```

If you would like to install from source:

```bash
git clone https://github.com/jmwoliver/hui.git
cd hui
cargo build
```

### Configuration

After installing `hui`, it will need to know which shell you are using. This can be done by setting the `HUI_TERM` environment variable.

For `zsh`, run the following commands:

```bash
echo 'export HUI_TERM="zsh"' >> ~/.zshrc
source ~/.zshrc
```

If you are using `bash`, run:

```bash
echo 'export HUI_TERM="bash"' >> ~/.bashrc
source ~/.bashrc
```

## Usage

Once everything is installed and the `HUI_TERM` environment variable is set, all you have to do to run it is:

```
hui
```

Now you can scroll through all your history, filter results, and select a command to copy to your clipboard.

Enjoy!
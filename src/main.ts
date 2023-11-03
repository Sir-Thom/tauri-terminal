import { Terminal } from "xterm";
import { FitAddon } from "xterm-addon-fit";
import "xterm/css/xterm.css";
import { invoke } from "@tauri-apps/api";
import { I } from "@tauri-apps/api/path-c062430b";

const terminalElement = document.getElementById("terminal") as HTMLElement;


const fitAddon = new FitAddon();
const term = new Terminal({
  fontFamily: "Jetbrains Mono",
  theme: {
    background: "rgb(47, 47, 47)",
  },
});
term.loadAddon(fitAddon);
term.open(terminalElement);

// Make the terminal fit all the window size
async function fitTerminal() {
  fitAddon.fit();
  void invoke<string>("async_resize_pty", {
    ptyIndex: 0,
    rows: term.rows,
    cols: term.cols,
  });
}

// Write data from pty into the terminal
function writeToTerminal(data: string) {
  return new Promise<void>((r) => {
    term.write(data, () => r());
  });
}

// Write data from the terminal to the pty
function writeToPty(ptyIndex: number, data: string) {
  console.log(`Writing to PTY: ${data}`);
  void invoke("async_write_to_pty", {
    ptyIndex,
    data,
  });
}

function createWriteToPtyHandler(ptyIndex: number) {
  return function(data: string) {
    writeToPty(ptyIndex, data);
  };
}
function createResizePtyHandler(ptyIndex: number) {
  return function() {
    resizePty(ptyIndex, term.rows, term.cols);
  };
}

// then, when you initialize the shell:

async function initShell() {
  const ptyIndex = await invoke<number>("async_add_pty");
  console.log("ptyIndex", ptyIndex);
  await invoke("async_create_shell", { ptyIndex: ptyIndex }).catch((error) => {
    console.error("Error creating shell:", error);
  });
  console.log("Shell created");
  term.onData(createWriteToPtyHandler(ptyIndex));
  window.addEventListener("resize", createResizePtyHandler(ptyIndex));
  window.requestAnimationFrame(() => readFromPty(ptyIndex));
}







async function readFromPty(ptyIndex: number) {
  const data = await invoke<string>("async_read_from_pty", { ptyIndex });

  if (data) {
    console.log("Received from PTY:", data);
    await writeToTerminal(data);
  }

  window.requestAnimationFrame(() => readFromPty(ptyIndex));
}

async function resizePty(ptyIndex: number, rows: number, cols: number) {
  await invoke("async_resize_pty", {
    ptyIndex,
    rows,
    cols,
  });
}


initShell().then(() => {
  console.log("Shell initialized");
  fitTerminal();
});
writeToTerminal('Hello, World!');




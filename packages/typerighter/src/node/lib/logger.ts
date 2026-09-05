import fs from 'node:fs';
import pc from 'picocolors';

const ANSI_RE = /\u001B\[[0-9;]*m/g;

export class TdLogger {
  private logStream: fs.WriteStream | undefined;

  constructor (logFilePath?: string) {
    if (logFilePath !== undefined) {
      this.logStream = fs.createWriteStream(logFilePath, {
        flags: 'a',
      });
      process.once('exit', () => this.close());
    }
  }

  info (message: string) {
    console.log(pc.cyan(`ℹ  ${message}`));
    this.writeToFile('INFO', message);
  }

  warn (message: string) {
    console.warn(pc.yellow(`⚠  ${message}`));
    this.writeToFile('WARN', message);
  }

  error (message: string) {
    console.error(pc.red(`✖  ${message}`));
    this.writeToFile('ERROR', message);
  }

  success (message: string) {
    console.log(pc.green(`✔  ${message}`));
    this.writeToFile('SUCCESS', message);
  }

  start (message: string) {
    console.log(pc.cyan(`◐  ${message}`));
    this.writeToFile('START', message);
  }

  log (message: string) {
    console.log(message);
    this.writeToFile('LOG', message);
  }

  writeToFile (level: string, message: string) {
    if (!this.logStream) return;

    const time = new Date().toISOString();
    const plain = message.replace(ANSI_RE, '');

    this.logStream.write(`${time} [${level}] ${plain}\n`);
  }

  close () {
    this.logStream?.end();
    this.logStream = undefined;
  }
}

export const consoleLogger = new TdLogger();

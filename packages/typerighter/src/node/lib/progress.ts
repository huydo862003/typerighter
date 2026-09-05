import pc from 'picocolors';
import type {
  TdLogger,
} from './logger';

const SPINNER_FRAMES = ['◐', '◓', '◑', '◒'];

export class ProgressLogger {
  private logger: TdLogger;
  private label: string;
  private start: number;
  private spinnerInterval: ReturnType<typeof setInterval> | undefined;

  constructor (logger: TdLogger, label: string) {
    this.logger = logger;
    this.label = label;
    this.start = performance.now();

    if (process.stdout.isTTY) {
      let frame = 0;

      process.stdout.write(pc.cyan(`${SPINNER_FRAMES[0]} ${label}`));
      this.spinnerInterval = setInterval(() => {
        frame = (frame + 1) % SPINNER_FRAMES.length;
        process.stdout.write(`\r${pc.cyan(`${SPINNER_FRAMES[frame]} ${this.label}`)}`);
      }, 120).unref();
    } else {
      this.logger.start(label);
    }
  }

  update (current: number, total: number) {
    if (!process.stdout.isTTY) return;

    const pct = Math.round((current / total) * 100);

    this.stopSpinner();
    process.stdout.write(`\r  ${this.label} ${current}/${total} (${pct}%)`);
  }

  done (message: string) {
    this.stopSpinner();

    if (process.stdout.isTTY) {
      process.stdout.write('\r\x1b[K');
    }

    const elapsed = Math.round(performance.now() - this.start);

    this.logger.success(`${message} (${formatMs(elapsed)})`);
  }

  private stopSpinner () {
    if (this.spinnerInterval) {
      clearInterval(this.spinnerInterval);
      this.spinnerInterval = undefined;
    }
  }
}

function formatMs (ms: number): string {
  return ms < 1000 ? `${Math.round(ms)}ms` : `${(ms / 1000).toFixed(1)}s`;
}

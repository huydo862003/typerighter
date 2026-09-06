/**
 * JSON-RPC 2.0 message types and error codes
 * Copied from vscode-jsonrpc (MIT, Microsoft Corporation)
 * Ref: https://github.com/microsoft/vscode-languageserver-node/tree/main/jsonrpc/src/common/messages
 */

// https://github.com/microsoft/vscode-languageserver-node/blob/5010cdf9822e1038a30ee7eb6ee5d7aaa79acc4a/jsonrpc/src/common/messages.ts#L11
export interface Message {
  jsonrpc: string;
}

// https://github.com/microsoft/vscode-languageserver-node/blob/5010cdf9822e1038a30ee7eb6ee5d7aaa79acc4a/jsonrpc/src/common/messages.ts#L18
export interface RequestMessage extends Message {
  id: number | string | null;
  method: string;
  params?: any[] | object;
}

// https://github.com/microsoft/vscode-languageserver-node/blob/5010cdf9822e1038a30ee7eb6ee5d7aaa79acc4a/jsonrpc/src/common/messages.ts#L150-L165
export interface ResponseMessage extends Message {
  id: number | string | null;
  result?: string | number | boolean | object | any[] | null;
  error?: ResponseErrorLiteral<any>;
}

// https://github.com/microsoft/vscode-languageserver-node/blob/5010cdf9822e1038a30ee7eb6ee5d7aaa79acc4a/jsonrpc/src/common/messages.ts#L283-L292
export interface NotificationMessage extends Message {
  method: string;
  params?: any[] | object;
}

// https://github.com/microsoft/vscode-languageserver-node/blob/5010cdf9822e1038a30ee7eb6ee5d7aaa79acc4a/jsonrpc/src/common/messages.ts#L105-L122
export interface ResponseErrorLiteral<D = void> {
  code: number;
  message: string;
  data?: D;
}

// https://github.com/microsoft/vscode-languageserver-node/blob/5010cdf9822e1038a30ee7eb6ee5d7aaa79acc4a/jsonrpc/src/common/messages.ts#L298-L323
export namespace Message {
  export function isRequest (message: Message | undefined): message is RequestMessage {
    const candidate = message as RequestMessage;
    return candidate !== undefined && typeof candidate.method === 'string'
      && (typeof candidate.id === 'string' || typeof candidate.id === 'number');
  }

  export function isNotification (message: Message | undefined): message is NotificationMessage {
    const candidate = message as NotificationMessage;
    return candidate !== undefined && typeof candidate.method === 'string'
      && (candidate as any).id === undefined;
  }

  export function isResponse (message: Message | undefined): message is ResponseMessage {
    const candidate = message as ResponseMessage;
    return candidate !== undefined
      && (candidate.result !== undefined || !!candidate.error)
      && (typeof candidate.id === 'string' || typeof candidate.id === 'number' || candidate.id === null);
  }
}

// https://github.com/microsoft/vscode-languageserver-node/blob/5010cdf9822e1038a30ee7eb6ee5d7aaa79acc4a/jsonrpc/src/common/messages.ts#L32-L83
export namespace ErrorCodes {
  export const ParseError: -32700 = -32700;
  export const InvalidRequest: -32600 = -32600;
  export const MethodNotFound: -32601 = -32601;
  export const InvalidParams: -32602 = -32602;
  export const InternalError: -32603 = -32603;

  // JSON-RPC reserved error range (-32099 to -32000)
  export const jsonrpcReservedErrorRangeStart: -32099 = -32099;
  export const MessageWriteError: -32099 = -32099;
  export const MessageReadError: -32098 = -32098;
  export const PendingResponseRejected: -32097 = -32097;
  export const ConnectionInactive: -32096 = -32096;
  export const ServerNotInitialized: -32002 = -32002;
  export const UnknownErrorCode: -32001 = -32001;
  export const jsonrpcReservedErrorRangeEnd: -32000 = -32000;
}

// https://github.com/microsoft/vscode-languageserver-node/blob/5010cdf9822e1038a30ee7eb6ee5d7aaa79acc4a/jsonrpc/src/common/messages.ts#L128-L148
export class ResponseError<D = void> extends Error {
  readonly code: number;
  readonly data: D | undefined;

  constructor (code: number, message: string, data?: D) {
    super(message);
    this.code = typeof code === 'number' ? code : ErrorCodes.UnknownErrorCode;
    this.data = data;
    Object.setPrototypeOf(this, ResponseError.prototype);
  }

  toJson (): ResponseErrorLiteral<D> {
    const result: ResponseErrorLiteral<D> = {
      code: this.code,
      message: this.message,
    };

    if (this.data !== undefined) {
      result.data = this.data;
    }

    return result;
  }
}

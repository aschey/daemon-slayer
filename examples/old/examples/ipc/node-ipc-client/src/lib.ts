import { createConnection, Socket } from "net";
import { platform } from "os";

export class Client<Req, Res> {
  #connection: Socket;

  constructor(connection: Socket) {
    this.#connection = connection;
  }

  static connect = async <Req, Res>(appName: string) => {
    let connection = await new Promise<Socket>((resolve) => {
      const connection = createConnection(getSocketAddress(appName), () => {
        resolve(connection);
      });
    });

    return new Client<Req, Res>(connection);
  };

  async send(req: Req) {
    // Initiate the data event handler before sending the request to ensure we don't miss the response
    const data = this.#read();
    const reqBytes = Buffer.from(JSON.stringify(req));
    await this.#write(intToBytes(reqBytes.length));
    await this.#write(reqBytes);

    return await data;
  }

  async #read() {
    const data = new Promise<Res>((resolve) => {
      this.#connection.on("data", (data) => {
        // Ensure that we clean up the handler on every request since they don't get removed automatically
        this.#connection.removeAllListeners("data");
        const int32ByteLength = 4;
        resolve(JSON.parse(data.subarray(int32ByteLength).toString()));
      });
    });
    return data;
  }

  async #write(buf: Buffer) {
    await new Promise((resolve, reject) => {
      this.#connection.write(buf, (err) => {
        if (err) {
          reject(err);
        } else {
          resolve(undefined);
        }
      });
    });
  }
}

const getSocketAddress = (appName: string) => {
  if (platform() === "win32") {
    return `\\\\.\\pipe\\${appName}`;
  } else {
    return `/tmp/${appName}.sock`;
  }
};

const intToBytes = (i: number) => {
  let buf = Buffer.alloc(4);
  buf.writeUInt32BE(i);
  return buf;
};

export const sleep = (milliseconds: number) => {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
};

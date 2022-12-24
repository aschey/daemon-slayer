import { Client } from "./lib.js";

interface ipcRequest {
  name: string;
}

interface ipcResponse {
  message: string;
}

const sleep = (milliseconds: number) => {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
};

const client = await Client.connect<ipcRequest, ipcResponse>(
  "daemon_slayer_ipc"
);

while (true) {
  const response = await client.send({ name: "bob" });

  console.log(response);
  await sleep(1000);
}

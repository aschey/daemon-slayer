package main

import (
	"log"
	"time"
)

type ipcRequest struct {
	Name string `json:"name"`
}

type ipcResponse struct {
	Message string `json:"message"`
}

func main() {
	cc, err := StartClient[ipcRequest, ipcResponse]("daemon_slayer_ipc_ipc")
	if err != nil {
		return
	}

	for {
		err = cc.Write(ipcRequest{Name: "bob"})
		if err != nil {
			log.Println(err)
			return
		}
		msg, err := cc.Read()
		if err != nil {
			log.Println(err)
			return
		}

		log.Println(msg)
		time.Sleep(time.Second)

	}
}

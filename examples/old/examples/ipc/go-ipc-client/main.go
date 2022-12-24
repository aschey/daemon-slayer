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
	cc, err := StartClient[ipcRequest, ipcResponse]("daemon_slayer_ipc")
	if err != nil {
		return
	}

	for {
		msg, err := cc.Send(ipcRequest{Name: "bob"})
		if err != nil {
			log.Println(err)
			return
		}

		log.Println(msg)
		time.Sleep(time.Second)

	}
}

//go:build linux || darwin

package main

import (
	"net"
	"strings"
	"time"
)

func (cc *Client[Req, Res]) dial() error {

	base := "/tmp/"
	sock := ".sock"

	for {

		conn, err := net.Dial("unix", base+cc.Name+sock)
		if err != nil {
			if !(strings.Contains(err.Error(), "connect: no such file or directory") ||
				strings.Contains(err.Error(), "connect: connection refused")) {
				return err
			}
		} else {
			cc.conn = conn
			return nil
		}

		time.Sleep(cc.retryTimer * time.Second)
	}

}

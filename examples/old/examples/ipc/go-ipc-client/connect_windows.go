package main

import (
	"strings"
	"time"

	"github.com/Microsoft/go-winio"
)

func (cc *Client[Req, Res]) dial() error {

	var pipeBase = `\\.\pipe\`

	for {
		pn, err := winio.DialPipe(pipeBase+cc.Name, nil)

		if err != nil {
			if !strings.Contains(err.Error(), "The system cannot find the file specified.") {
				return err
			}
		} else {
			cc.conn = pn
			return nil
		}

		time.Sleep(cc.retryTimer)
	}
}

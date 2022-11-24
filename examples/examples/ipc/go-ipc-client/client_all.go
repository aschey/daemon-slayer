package main

import (
	"bufio"
	"bytes"
	"encoding/binary"
	"encoding/json"
	"net"
	"time"
)

type Client[Req any, Res any] struct {
	Name string
	conn net.Conn

	retryTimer time.Duration
}

func StartClient[Req any, Res any](ipcName string) (*Client[Req, Res], error) {

	cc := &Client[Req, Res]{
		Name:       ipcName,
		retryTimer: time.Duration(100 * time.Millisecond),
	}

	err := cc.dial()
	if err != nil {
		return nil, err
	}

	return cc, nil

}

func bytesToInt(b []byte) (int, error) {

	var mlen uint32

	if err := binary.Read(bytes.NewReader(b[:]), binary.BigEndian, &mlen); err != nil {
		return 0, err
	}
	return int(mlen), nil

}

func intToBytes(mLen int) []byte {

	b := make([]byte, 4)
	binary.BigEndian.PutUint32(b, uint32(mLen))

	return b

}

func (cc *Client[Req, Res]) Send(request Req) (Res, error) {
	if err := cc.write(request); err != nil {
		var res Res
		return res, err
	}
	return cc.read()
}

func (cc *Client[Req, Res]) read() (Res, error) {
	bLen := make([]byte, 4)
	var res Res
	if _, err := cc.conn.Read(bLen); err != nil {
		return res, err
	}

	mLen, err := bytesToInt(bLen)
	if err != nil {
		return res, err
	}
	message := make([]byte, mLen)

	_, err = cc.conn.Read(message)
	if err != nil {
		return res, err
	}

	err = json.Unmarshal(message, &res)
	return res, err
}

func (cc *Client[Req, Res]) write(message Req) error {
	data, err := json.Marshal(message)
	if err != nil {
		return err
	}

	writer := bufio.NewWriter(cc.conn)
	if _, err := writer.Write(intToBytes(len(data))); err != nil {
		return err
	}
	if _, err := writer.Write(data); err != nil {
		return err
	}

	err = writer.Flush()
	if err != nil {
		return err
	}

	return nil
}

func (cc *Client[Req, Res]) Close() {
	cc.conn.Close()
}

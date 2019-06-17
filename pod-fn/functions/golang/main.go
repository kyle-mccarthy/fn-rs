package main

import (
	"encoding/json"
	"os"
	"fmt"
	"log"
	"net"
	"os/signal"
	"syscall"
)

type FunctionPayload struct {
	Req FunctionRequest
	Res FunctionResponse
}

type FunctionRequest struct {
	Path string `json:"path"`
	Method string `json:"method"`
	Headers map[string]string `json:"headers"`
	QueryString string `json:"query_string"`
	Script string `json:"script"`
	Body string `json:"body"`
}

type FunctionResponse struct {
	StatusCode uint16 `json:"status_code"`
	Body string `json:"body"`
	Headers map[string]string `json:"headers"`
}

func main() {
	socketAddr := os.Args[1]
	connect(socketAddr)
}

// https://gist.github.com/hakobe/6f70d69b8c5243117787fd488ae7fbf2
func connect(addr string) {
	conn, err := net.Listen("unix", addr)

	if err != nil {
		log.Fatalf("Failed to listen on unix address %s :: cause %s", addr, err)
	}

	shutdown := make(chan os.Signal, 1)
	signal.Notify(shutdown, os.Interrupt, syscall.SIGTERM)

	go func(conn net.Listener, c chan os.Signal) {
		sig := <-c
		log.Printf("Caught shutdown signal %s", sig)
		err := conn.Close()
		log.Printf("Error while shutting down %s", err)
		os.Exit(0)
	}(conn, shutdown)

	for {
		fd, err := conn.Accept()

		if err != nil {
			log.Fatal("Error accepting: ", err)
		}

		go handleRequest(fd)
	}
}

func handleRequest(c net.Conn) {
	buf := make([]byte, 1024)
	bytesRead, err := c.Read(buf)

	if err != nil {
		log.Print("Error while reading bytes from socket: ", err)
		return
	}

	data := buf[0:bytesRead]

	payload := FunctionPayload{}

	err = json.Unmarshal(data, &payload)

	if err != nil {
		log.Println("Failed to unmarshal the incoming request: ", err)
		_ = c.Close()
		return
	}

	fmt.Printf("got data %s", data)

	payload.Res.Body = "hello from go"

	res, err := json.Marshal(&payload.Res)

	if err != nil {
		log.Println("Failed to marshal the response: ", err)
		_ = c.Close()
		return
	}

	_, err = c.Write(res)

	if err != nil {
		log.Fatal("Error while writing bytes for client: ", err)
	}
}
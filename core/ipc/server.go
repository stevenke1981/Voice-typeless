// Package ipc implements the JSON-RPC 2.0 sidecar server that bridges
// Tauri (Rust) commands to the Go Core library over a TCP socket / named pipe.
package ipc

import (
	"encoding/json"
	"log"
	"net"
)

// HandlerFunc processes a JSON-RPC request and returns a result or error.
type HandlerFunc func(params json.RawMessage) (any, error)

// Server listens for JSON-RPC requests from the Tauri sidecar host.
type Server struct {
	addr     string
	handlers map[string]HandlerFunc
}

// NewServer creates a new IPC server that will listen on addr (e.g. "127.0.0.1:0").
func NewServer(addr string) *Server {
	return &Server{addr: addr, handlers: make(map[string]HandlerFunc)}
}

// Register adds a handler for the given method name.
func (s *Server) Register(method string, fn HandlerFunc) {
	s.handlers[method] = fn
}

// Start begins listening and serving requests. Blocks until the listener errors.
func (s *Server) Start() error {
	ln, err := net.Listen("tcp", s.addr)
	if err != nil {
		return err
	}
	log.Printf("ipc: listening on %s", ln.Addr())
	for {
		conn, err := ln.Accept()
		if err != nil {
			return err
		}
		go s.handleConn(conn)
	}
}

func (s *Server) handleConn(conn net.Conn) {
	defer conn.Close()
	dec := json.NewDecoder(conn)
	enc := json.NewEncoder(conn)

	for {
		var req RPCRequest
		if err := dec.Decode(&req); err != nil {
			return
		}
		handler, ok := s.handlers[req.Method]
		if !ok {
			_ = enc.Encode(RPCResponse{
				JSONRPC: "2.0",
				Error:   &RPCError{Code: -32601, Message: "method not found"},
				ID:      req.ID,
			})
			continue
		}
		result, err := handler(req.Params)
		if err != nil {
			_ = enc.Encode(RPCResponse{
				JSONRPC: "2.0",
				Error:   &RPCError{Code: -32000, Message: err.Error()},
				ID:      req.ID,
			})
			continue
		}
		_ = enc.Encode(RPCResponse{JSONRPC: "2.0", Result: result, ID: req.ID})
	}
}

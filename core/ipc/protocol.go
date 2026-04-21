package ipc

import "encoding/json"

// RPCRequest is a JSON-RPC 2.0 request message.
type RPCRequest struct {
	JSONRPC string          `json:"jsonrpc"`
	Method  string          `json:"method"`
	Params  json.RawMessage `json:"params"`
	ID      any             `json:"id"`
}

// RPCResponse is a JSON-RPC 2.0 response message.
type RPCResponse struct {
	JSONRPC string    `json:"jsonrpc"`
	Result  any       `json:"result,omitempty"`
	Error   *RPCError `json:"error,omitempty"`
	ID      any       `json:"id"`
}

// RPCError carries JSON-RPC error details.
type RPCError struct {
	Code    int    `json:"code"`
	Message string `json:"message"`
	Data    any    `json:"data,omitempty"`
}

// RPCEvent is a push notification sent by the Go sidecar to Rust (id is null).
type RPCEvent struct {
	JSONRPC string `json:"jsonrpc"`
	Method  string `json:"method"` // always "event"
	Params  struct {
		Name    string `json:"name"`
		Payload any    `json:"payload"`
	} `json:"params"`
}

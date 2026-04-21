// Package history persists recognition results using SQLite (pure Go, no CGO).
package history

import "time"

// HistoryItem is a single recognition entry.
type HistoryItem struct {
	ID        string
	Text      string
	Language  string
	Timestamp time.Time
}

// HistoryStore manages the recognition history.
type HistoryStore interface {
	// Add inserts a new item. ID is assigned by the store.
	Add(text, language string) (*HistoryItem, error)

	// List returns the most recent n items, newest first.
	List(n int) ([]HistoryItem, error)

	// Delete removes a single item by ID.
	Delete(id string) error

	// Prune removes items older than retentionDays.
	Prune(retentionDays int) error

	// Close releases the database connection.
	Close() error
}

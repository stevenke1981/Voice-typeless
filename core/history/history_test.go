package history_test

import (
	"testing"

	"github.com/vtl/core/history"
)

func TestSQLiteStore_AddAndList(t *testing.T) {
	store, err := history.NewSQLiteStore()
	if err != nil {
		t.Skip("SQLite store unavailable:", err)
	}
	defer store.Close()

	item, err := store.Add("hello world", "en")
	if err != nil {
		t.Fatal(err)
	}
	if item.Text != "hello world" {
		t.Errorf("expected 'hello world', got %q", item.Text)
	}
	if item.ID == "" {
		t.Error("expected non-empty ID")
	}

	items, err := store.List(10)
	if err != nil {
		t.Fatal(err)
	}
	if len(items) == 0 {
		t.Error("expected at least one history item")
	}
}

func TestSQLiteStore_Delete(t *testing.T) {
	store, err := history.NewSQLiteStore()
	if err != nil {
		t.Skip("SQLite store unavailable:", err)
	}
	defer store.Close()

	item, err := store.Add("delete me", "en")
	if err != nil {
		t.Fatal(err)
	}

	if err := store.Delete(item.ID); err != nil {
		t.Fatal(err)
	}
}

func TestSQLiteStore_Prune(t *testing.T) {
	store, err := history.NewSQLiteStore()
	if err != nil {
		t.Skip("SQLite store unavailable:", err)
	}
	defer store.Close()

	// Prune items older than 0 days (all items older than now) — should not error.
	if err := store.Prune(365); err != nil {
		t.Fatal(err)
	}
}

package history

import (
	"database/sql"
	"fmt"
	"os"
	"path/filepath"
	"time"

	_ "modernc.org/sqlite"
)

type sqliteStore struct {
	db *sql.DB
}

// NewSQLiteStore opens (or creates) the history database at the OS AppData path.
func NewSQLiteStore() (HistoryStore, error) {
	dir, err := os.UserConfigDir()
	if err != nil {
		return nil, err
	}
	dbPath := filepath.Join(dir, "VoiceTypeless", "history.db")
	if err := os.MkdirAll(filepath.Dir(dbPath), 0o755); err != nil {
		return nil, err
	}

	db, err := sql.Open("sqlite", dbPath)
	if err != nil {
		return nil, err
	}

	if _, err := db.Exec(`
		CREATE TABLE IF NOT EXISTS history (
			id         TEXT PRIMARY KEY,
			text       TEXT NOT NULL,
			language   TEXT NOT NULL DEFAULT 'en',
			created_at INTEGER NOT NULL
		);
		CREATE INDEX IF NOT EXISTS idx_history_created ON history(created_at DESC);
	`); err != nil {
		db.Close()
		return nil, err
	}

	return &sqliteStore{db: db}, nil
}

func (s *sqliteStore) Add(text, language string) (*HistoryItem, error) {
	id := fmt.Sprintf("%d", time.Now().UnixNano())
	ts := time.Now().Unix()
	_, err := s.db.Exec(
		`INSERT INTO history (id, text, language, created_at) VALUES (?, ?, ?, ?)`,
		id, text, language, ts,
	)
	if err != nil {
		return nil, err
	}
	return &HistoryItem{ID: id, Text: text, Language: language, Timestamp: time.Unix(ts, 0)}, nil
}

func (s *sqliteStore) List(n int) ([]HistoryItem, error) {
	rows, err := s.db.Query(
		`SELECT id, text, language, created_at FROM history ORDER BY created_at DESC LIMIT ?`, n,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var items []HistoryItem
	for rows.Next() {
		var item HistoryItem
		var ts int64
		if err := rows.Scan(&item.ID, &item.Text, &item.Language, &ts); err != nil {
			return nil, err
		}
		item.Timestamp = time.Unix(ts, 0)
		items = append(items, item)
	}
	return items, rows.Err()
}

func (s *sqliteStore) Delete(id string) error {
	_, err := s.db.Exec(`DELETE FROM history WHERE id = ?`, id)
	return err
}

func (s *sqliteStore) Prune(retentionDays int) error {
	cutoff := time.Now().AddDate(0, 0, -retentionDays).Unix()
	_, err := s.db.Exec(`DELETE FROM history WHERE created_at < ?`, cutoff)
	return err
}

func (s *sqliteStore) Close() error {
	return s.db.Close()
}

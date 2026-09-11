#pragma once
#include <QObject>

// Session-scoped view state persisted in session.json (never config.toml).
// Currently just the zoom remembered for each view so a restart keeps the
// column count / row height the user left them at. Values default to 0,
// meaning "nothing saved yet": the views then keep their built-in defaults.
class SessionState : public QObject
{
    Q_OBJECT
    Q_PROPERTY(int gridColumns READ gridColumns WRITE setGridColumns NOTIFY gridColumnsChanged)
    Q_PROPERTY(int rowHeightDetailed READ rowHeightDetailed WRITE setRowHeightDetailed NOTIFY rowHeightDetailedChanged)
    Q_PROPERTY(int rowHeightMiller READ rowHeightMiller WRITE setRowHeightMiller NOTIFY rowHeightMillerChanged)

public:
    explicit SessionState(QObject *parent = nullptr) : QObject(parent) {}

    int gridColumns() const { return m_gridColumns; }
    void setGridColumns(int value);

    int rowHeightDetailed() const { return m_rowHeightDetailed; }
    void setRowHeightDetailed(int value);

    int rowHeightMiller() const { return m_rowHeightMiller; }
    void setRowHeightMiller(int value);

signals:
    void gridColumnsChanged();
    void rowHeightDetailedChanged();
    void rowHeightMillerChanged();

private:
    int m_gridColumns = 0;
    int m_rowHeightDetailed = 0;
    int m_rowHeightMiller = 0;
};

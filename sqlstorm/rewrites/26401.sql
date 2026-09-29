WITH UserReputation AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        u.Reputation,
        COUNT(DISTINCT p.Id) AS PostCount,
        SUM(COALESCE(vb.VoteCount, 0)) AS UpVotes,
        SUM(COALESCE(vd.VoteCount, 0)) AS DownVotes
    FROM Users u
    LEFT JOIN Posts p ON u.Id = p.OwnerUserId
    LEFT JOIN (
        SELECT PostId, COUNT(*) AS VoteCount
        FROM Votes
        WHERE VoteTypeId = 2  
        GROUP BY PostId
    ) vb ON p.Id = vb.PostId
    LEFT JOIN (
        SELECT PostId, COUNT(*) AS VoteCount
        FROM Votes
        WHERE VoteTypeId = 3  
        GROUP BY PostId
    ) vd ON p.Id = vd.PostId
    GROUP BY u.Id, u.DisplayName, u.Reputation
),
PostStatistics AS (
    SELECT
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        EXTRACT(MONTH FROM p.CreationDate) AS MonthCreated,
        EXTRACT(YEAR FROM p.CreationDate) AS YearCreated,
        COUNT(c.Id) AS CommentCount,
        COUNT(DISTINCT ph.Id) AS EditCount,
        COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId = 10 THEN ph.Id END) AS CloseCount,
        COUNT(DISTINCT CASE WHEN ph.PostHistoryTypeId = 11 THEN ph.Id END) AS ReopenCount,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS TotalUpVotes,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS TotalDownVotes
    FROM Posts p
    LEFT JOIN Comments c ON p.Id = c.PostId
    LEFT JOIN PostHistory ph ON p.Id = ph.PostId
    LEFT JOIN Votes v ON p.Id = v.PostId
    GROUP BY p.Id, p.Title, p.CreationDate
),
UserPostActivity AS (
    SELECT
        ur.DisplayName,
        ps.PostId,
        ps.Title,
        ps.CreationDate,
        ps.CommentCount,
        ps.EditCount,
        ps.CloseCount,
        ps.ReopenCount,
        ps.TotalUpVotes,
        ps.TotalDownVotes,
        ur.Reputation
    FROM PostStatistics ps
    JOIN Users u ON ps.PostId = u.Id
    JOIN UserReputation ur ON u.Id = ur.UserId
),
FinalStatistics AS (
    SELECT 
        upa.DisplayName,
        upa.PostId,
        upa.Title,
        upa.CreationDate,
        upa.CommentCount,
        upa.EditCount,
        upa.CloseCount,
        upa.ReopenCount,
        upa.TotalUpVotes,
        upa.TotalDownVotes,
        upa.Reputation,
        ROW_NUMBER() OVER (PARTITION BY upa.DisplayName ORDER BY upa.CreationDate DESC, upa.PostId) AS RowNum
    FROM UserPostActivity upa
)

SELECT DisplayName, Title, CreationDate, CommentCount, EditCount, CloseCount, ReopenCount, TotalUpVotes, TotalDownVotes, Reputation, RowNum
FROM FinalStatistics
WHERE RowNum = 1
ORDER BY Reputation DESC, TotalUpVotes DESC;

WITH RankedPosts AS (
    SELECT
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.Score,
        p.AnswerCount,
        p.OwnerUserId,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id) AS RowNum,
        COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 2) OVER (PARTITION BY p.Id) AS UpvoteCount,
        COUNT(v.Id) FILTER (WHERE v.VoteTypeId = 3) OVER (PARTITION BY p.Id) AS DownvoteCount,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) OVER (PARTITION BY p.ParentId) AS ParentUpvotes
    FROM Posts p
    LEFT JOIN Votes v ON p.Id = v.PostId
    WHERE p.CreationDate > (CAST('2024-10-01 12:34:56' AS TIMESTAMP) - INTERVAL '1 year') 
      AND p.Score IS NOT NULL
),
UserStats AS (
    SELECT
        u.Id AS UserId,
        u.DisplayName,
        COUNT(b.Id) AS BadgeCount,
        SUM(COALESCE(u.UpVotes, 0) - COALESCE(u.DownVotes, 0)) AS ReputationDelta,
        MAX(b.Class) AS HighestBadgeClass
    FROM Users u
    LEFT JOIN Badges b ON u.Id = b.UserId
    GROUP BY u.Id, u.DisplayName
),
ClosedPosts AS (
    SELECT 
        ph.PostId, 
        COUNT(*) AS CloseCount,
        STRING_AGG(DISTINCT ph.Comment, ', ') AS CloseReasons
    FROM PostHistory ph
    WHERE ph.PostHistoryTypeId IN (10, 11) 
    GROUP BY ph.PostId
)
SELECT 
    rp.Title,
    us.DisplayName,
    rp.CreationDate,
    rp.Score,
    rp.AnswerCount,
    rp.UpvoteCount,
    rp.DownvoteCount,
    rp.ParentUpvotes,
    us.BadgeCount,
    COALESCE(cp.CloseCount, 0) AS CloseCount,
    cp.CloseReasons
FROM RankedPosts rp
JOIN UserStats us ON rp.OwnerUserId = us.UserId
LEFT JOIN ClosedPosts cp ON rp.PostId = cp.PostId
WHERE rp.RowNum = 1 
  AND (rp.Score BETWEEN 10 AND 100 OR rp.AnswerCount > 5) 
  AND COALESCE(cp.CloseCount, 0) < 2 
ORDER BY rp.CreationDate DESC
LIMIT 50;

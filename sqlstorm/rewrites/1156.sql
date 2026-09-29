
WITH UserActivity AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        COUNT(DISTINCT p.Id) AS PostCount,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS Upvotes,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS Downvotes,
        SUM(epoch_us(p.LastActivityDate) - epoch_us(p.CreationDate))::DOUBLE / COUNT(p.Id) / 1e6 AS AvgResponseTime
    FROM Users u
    LEFT JOIN Posts p ON u.Id = p.OwnerUserId
    LEFT JOIN Votes v ON p.Id = v.PostId
    WHERE u.Reputation >= 100
    GROUP BY u.Id, u.DisplayName
),
TopPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.Score,
        u.DisplayName AS OwnerName,
        ROW_NUMBER() OVER (PARTITION BY p.PostTypeId ORDER BY p.Score DESC) AS RankedPosts
    FROM Posts p
    INNER JOIN Users u ON p.OwnerUserId = u.Id
    WHERE p.CreationDate > CURRENT_DATE - INTERVAL '90 days'
)
SELECT 
    ua.DisplayName,
    ua.PostCount,
    ua.Upvotes,
    ua.Downvotes,
    ua.AvgResponseTime,
    tp.Title,
    tp.CreationDate AS PostCreationDate,
    tp.Score,
    tp.OwnerName
FROM UserActivity ua
LEFT JOIN TopPosts tp ON ua.DisplayName = tp.OwnerName
WHERE (ua.Upvotes - ua.Downvotes) > 10
  AND (tp.RankedPosts <= 5 OR tp.RankedPosts IS NULL)
ORDER BY ua.Upvotes DESC, ua.AvgResponseTime ASC;

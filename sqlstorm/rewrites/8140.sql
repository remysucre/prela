
WITH PostStats AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.Score,
        p.ViewCount,
        p.AnswerCount,
        p.CommentCount,
        p.FavoriteCount,
        u.DisplayName AS OwnerDisplayName,
        COUNT(DISTINCT v.Id) AS VoteCount,
        AVG(c.Score) AS AverageCommentScore
    FROM 
        Posts p
    JOIN 
        Users u ON p.OwnerUserId = u.Id
    LEFT JOIN 
        Votes v ON p.Id = v.PostId
    LEFT JOIN 
        Comments c ON p.Id = c.PostId
    WHERE 
        p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
    GROUP BY 
        p.Id, p.Title, p.CreationDate, p.Score, p.ViewCount, p.AnswerCount, 
        p.CommentCount, p.FavoriteCount, u.DisplayName
),
UserBadgeStats AS (
    SELECT 
        b.UserId,
        COUNT(b.Id) AS BadgeCount,
        MAX(b.Class) AS HighestBadgeClass
    FROM 
        Badges b
    GROUP BY 
        b.UserId
),
CombinedStats AS (
    SELECT 
        ps.PostId,
        ps.Title,
        ps.CreationDate,
        ps.Score,
        ps.ViewCount,
        ps.AnswerCount,
        ps.CommentCount,
        ps.FavoriteCount,
        ps.OwnerDisplayName,
        ps.VoteCount,
        ps.AverageCommentScore,
        ubs.BadgeCount,
        ubs.HighestBadgeClass
    FROM 
        PostStats ps
    LEFT JOIN 
        UserBadgeStats ubs ON ps.OwnerDisplayName = (SELECT DisplayName FROM Users WHERE Id = ubs.UserId)
)
SELECT 
    *,
    CASE 
        WHEN HighestBadgeClass = 1 THEN 'Gold'
        WHEN HighestBadgeClass = 2 THEN 'Silver'
        WHEN HighestBadgeClass = 3 THEN 'Bronze'
        ELSE 'No Badge'
    END AS HighestBadge
FROM 
    CombinedStats
ORDER BY 
    Score DESC, ViewCount DESC, PostId, BadgeCount, HighestBadgeClass
LIMIT 100;

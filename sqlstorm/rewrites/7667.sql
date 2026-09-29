WITH PostStatistics AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.Score,
        p.ViewCount,
        COUNT(DISTINCT c.Id) AS CommentCount,
        SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
        SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
        p.CreationDate,
        p.LastActivityDate,
        COALESCE(p.AcceptedAnswerId, 0) AS AcceptedAnswerId,
        COALESCE(b.UserId, 0) AS BadgeOwnerId
    FROM 
        Posts p
    LEFT JOIN 
        Comments c ON p.Id = c.PostId
    LEFT JOIN 
        Votes v ON p.Id = v.PostId
    LEFT JOIN 
        Badges b ON b.UserId = p.OwnerUserId
    WHERE 
        p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR'
    GROUP BY 
        p.Id, p.Title, p.Score, p.ViewCount, p.CreationDate, p.LastActivityDate, p.AcceptedAnswerId, b.UserId
),
UserReputation AS (
    SELECT 
        u.Id AS UserId,
        u.Reputation,
        COUNT(b.Id) AS BadgeCount
    FROM 
        Users u
    LEFT JOIN 
        Badges b ON u.Id = b.UserId
    GROUP BY 
        u.Id, u.Reputation
)
SELECT 
    ps.PostId,
    ps.Title,
    ps.Score,
    ps.ViewCount,
    ps.CommentCount,
    ps.UpVotes,
    ps.DownVotes,
    ps.CreationDate,
    ps.LastActivityDate,
    ps.AcceptedAnswerId,
    ur.Reputation AS OwnerReputation,
    ur.BadgeCount
FROM 
    PostStatistics ps
JOIN 
    Users u ON ps.BadgeOwnerId = u.Id
JOIN 
    UserReputation ur ON u.Id = ur.UserId
ORDER BY 
    ps.Score DESC, ps.ViewCount DESC, ps.PostId
LIMIT 100;
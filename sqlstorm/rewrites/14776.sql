WITH UserBadges AS (
    SELECT 
        UserId,
        COUNT(*) AS BadgeCount
    FROM 
        Badges
    GROUP BY 
        UserId
),
PostDetails AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.Score,
        p.ViewCount,
        p.CreationDate AS PostCreationDate,
        u.Reputation,
        ub.BadgeCount,
        p.PostTypeId,
        p.AcceptedAnswerId,
        p.AnswerCount,
        p.CommentCount
    FROM 
        Posts p
    JOIN 
        Users u ON p.OwnerUserId = u.Id
    LEFT JOIN 
        UserBadges ub ON u.Id = ub.UserId
    WHERE 
        p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year' 
)
SELECT 
    p.PostId,
    p.Title,
    p.Score,
    p.ViewCount,
    p.PostCreationDate,
    p.Reputation,
    p.BadgeCount,
    p.PostTypeId,
    p.AcceptedAnswerId,
    p.AnswerCount,
    p.CommentCount
FROM 
    PostDetails p
ORDER BY 
    p.Score DESC, p.ViewCount DESC, p.PostId
LIMIT 100;
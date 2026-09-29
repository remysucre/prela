WITH LatestPosts AS (
    SELECT 
        p.Id,
        p.Title,
        p.CreationDate,
        p.OwnerUserId,
        p.AcceptedAnswerId,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id) AS rn
    FROM 
        Posts p
    WHERE 
        p.CreationDate > cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
),
UserReputation AS (
    SELECT 
        u.Id AS UserId,
        u.Reputation,
        COUNT(b.Id) AS BadgeCount,
        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
    FROM 
        Users u
    LEFT JOIN 
        Badges b ON u.Id = b.UserId
    GROUP BY 
        u.Id, u.Reputation
),
PostStats AS (
    SELECT 
        p.Id,
        COALESCE(v.UpVotes, 0) AS UpVotes,
        COALESCE(v.DownVotes, 0) AS DownVotes,
        COALESCE(mc.CommentCount, 0) AS CommentCount
    FROM 
        Posts p
    LEFT JOIN (
        SELECT 
            postId,
            SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
            SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
        FROM 
            Votes
        GROUP BY 
            postId
    ) v ON p.Id = v.postId
    LEFT JOIN (
        SELECT 
            PostId, 
            COUNT(*) AS CommentCount 
        FROM 
            Comments 
        GROUP BY 
            PostId
    ) mc ON p.Id = mc.PostId
)
SELECT 
    up.UserId,
    up.Reputation,
    lp.Title,
    lp.CreationDate AS PostDate,
    ps.UpVotes,
    ps.DownVotes,
    ps.CommentCount,
    (CASE 
        WHEN lp.AcceptedAnswerId IS NOT NULL THEN 'Yes' 
        ELSE 'No' 
    END) AS IsAcceptedAnswer,
    (SELECT 
        COUNT(*) FROM Posts p WHERE p.OwnerUserId = up.UserId AND p.PostTypeId = 2) AS AnswerCount,
    up.BadgeCount,
    up.GoldBadges,
    up.SilverBadges,
    up.BronzeBadges
FROM 
    UserReputation up
JOIN 
    LatestPosts lp ON up.UserId = lp.OwnerUserId
LEFT JOIN 
    PostStats ps ON lp.Id = ps.Id
WHERE 
    up.Reputation > (SELECT AVG(Reputation) FROM Users)
    AND lp.rn = 1
ORDER BY 
    up.Reputation DESC, PostDate DESC
LIMIT 100;

WITH UserBadgeStats AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        COUNT(b.Id) AS BadgeCount,
        SUM(CASE WHEN b.Class = 1 THEN 1 ELSE 0 END) AS GoldBadges,
        SUM(CASE WHEN b.Class = 2 THEN 1 ELSE 0 END) AS SilverBadges,
        SUM(CASE WHEN b.Class = 3 THEN 1 ELSE 0 END) AS BronzeBadges
    FROM 
        Users u
    LEFT JOIN 
        Badges b ON u.Id = b.UserId
    GROUP BY 
        u.Id, u.DisplayName
),
PopularPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        COUNT(c.Id) AS CommentCount,
        SUM(p.Score) AS TotalScore
    FROM 
        Posts p
    LEFT JOIN 
        Comments c ON p.Id = c.PostId
    WHERE 
        p.CreationDate >= '2022-01-01'  
    GROUP BY 
        p.Id, p.Title
    ORDER BY 
        TotalScore DESC, p.Id
    LIMIT 10  
),
BadgeAwardedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        ub.UserId,
        ub.DisplayName,
        ub.BadgeCount
    FROM 
        PopularPosts pp
    JOIN 
        Posts p ON p.Id = pp.PostId
    JOIN 
        UserBadgeStats ub ON ub.UserId = p.OwnerUserId
)
SELECT 
    bp.PostId,
    bp.Title,
    bp.DisplayName AS UserDisplayName,
    bp.BadgeCount,
    pp.CommentCount AS PostCommentCount,
    pp.TotalScore AS PostTotalScore
FROM 
    BadgeAwardedPosts bp
JOIN 
    PopularPosts pp ON bp.PostId = pp.PostId
ORDER BY 
    bp.BadgeCount DESC, pp.TotalScore DESC;

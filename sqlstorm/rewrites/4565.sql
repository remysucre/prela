
WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.CreationDate,
        p.Score,
        p.ViewCount,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.Id) AS UserPostRank,
        p.OwnerUserId
    FROM 
        Posts p
    WHERE 
        p.PostTypeId = 1 
        AND p.Score >= 0
),
UserBadges AS (
    SELECT 
        u.Id AS UserId,
        COUNT(b.Id) AS TotalBadges,
        MAX(b.Class) AS HighestBadgeClass
    FROM 
        Users u
    LEFT JOIN 
        Badges b ON u.Id = b.UserId
    GROUP BY 
        u.Id
),
PostCommentCounts AS (
    SELECT 
        c.PostId,
        COUNT(c.Id) AS CommentCount
    FROM 
        Comments c
    GROUP BY 
        c.PostId
),
FinalResults AS (
    SELECT 
        rp.PostId,
        rp.Title,
        rp.CreationDate,
        COALESCE(pcc.CommentCount, 0) AS CommentCount,
        ub.TotalBadges,
        ub.HighestBadgeClass,
        CASE 
            WHEN ub.HighestBadgeClass = 1 THEN 'Gold'
            WHEN ub.HighestBadgeClass = 2 THEN 'Silver'
            WHEN ub.HighestBadgeClass = 3 THEN 'Bronze'
            ELSE 'None'
        END AS BadgeLevel
    FROM 
        RankedPosts rp
    LEFT JOIN 
        UserBadges ub ON rp.OwnerUserId = ub.UserId
    LEFT JOIN 
        PostCommentCounts pcc ON rp.PostId = pcc.PostId
    WHERE 
        rp.UserPostRank <= 10
)
SELECT 
    PostId,
    Title,
    CreationDate,
    CommentCount,
    TotalBadges,
    BadgeLevel
FROM 
    FinalResults
ORDER BY 
    CreationDate DESC
LIMIT 50;

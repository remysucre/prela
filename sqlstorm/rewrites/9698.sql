
WITH UserBadges AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        u.Reputation,
        COUNT(DISTINCT b.Id) AS BadgeCount,
        SUM(CASE 
            WHEN b.Class = 1 THEN 1 
            ELSE 0 
        END) AS GoldBadges,
        SUM(CASE 
            WHEN b.Class = 2 THEN 1 
            ELSE 0 
        END) AS SilverBadges,
        SUM(CASE 
            WHEN b.Class = 3 THEN 1 
            ELSE 0 
        END) AS BronzeBadges
    FROM 
        Users u
    LEFT JOIN 
        Badges b ON u.Id = b.UserId
    GROUP BY 
        u.Id, u.DisplayName, u.Reputation
),
HighReputationUsers AS (
    SELECT 
        UserId,
        DisplayName,
        Reputation,
        BadgeCount,
        GoldBadges,
        SilverBadges,
        BronzeBadges
    FROM 
        UserBadges
    WHERE 
        Reputation > 1000
),
PopularPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.ViewCount,
        p.AnswerCount,
        p.CommentCount,
        p.CreationDate,
        p.OwnerUserId,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC, p.Id) AS Rank
    FROM 
        Posts p
    WHERE 
        p.PostTypeId = 1   
)
SELECT 
    bh.DisplayName AS Author,
    bh.Reputation,
    p.Title AS PostTitle,
    p.ViewCount AS Popularity,
    bh.GoldBadges,
    bh.SilverBadges,
    bh.BronzeBadges
FROM 
    HighReputationUsers bh
INNER JOIN 
    PopularPosts p ON bh.UserId = p.OwnerUserId
WHERE 
    p.Rank <= 5  
ORDER BY 
    bh.Reputation DESC, p.ViewCount DESC;

WITH RankedPosts AS (
    SELECT 
        p.Id,
        p.Title,
        p.CreationDate,
        p.OwnerUserId,
        p.Score,
        p.Tags,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC) as PostRank
    FROM 
        Posts p
    WHERE 
        p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
),
UserReputation AS (
    SELECT 
        u.Id AS UserId,
        u.Reputation, 
        COUNT(p.Id) AS PostCount
    FROM 
        Users u
    LEFT JOIN 
        Posts p ON u.Id = p.OwnerUserId
    GROUP BY 
        u.Id, u.Reputation
),
BadgeCounts AS (
    SELECT 
        b.UserId,
        COUNT(b.Id) AS BadgeCount
    FROM 
        Badges b
    GROUP BY 
        b.UserId
),
FinalResults AS (
    SELECT 
        up.Id AS PostId,
        up.Title,
        up.CreationDate,
        ur.Reputation,
        COALESCE(bc.BadgeCount, 0) AS BadgeCount,
        up.Score,
        up.Tags
    FROM 
        RankedPosts up
    LEFT JOIN 
        UserReputation ur ON up.OwnerUserId = ur.UserId
    LEFT JOIN 
        BadgeCounts bc ON up.OwnerUserId = bc.UserId
    WHERE 
        up.PostRank = 1
)
SELECT 
    fr.PostId,
    fr.Title,
    fr.CreationDate,
    fr.Reputation,
    fr.BadgeCount,
    fr.Score,
    fr.Tags
FROM 
    FinalResults fr
WHERE 
    fr.Reputation > (SELECT AVG(Reputation) FROM Users)
ORDER BY 
    fr.Score DESC, fr.PostId
LIMIT 10;